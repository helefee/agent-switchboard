//! Explicit, single-use requests against a saved provider or an in-memory draft.

mod claude;
mod connection;
mod contracts;
mod registry;
mod response;
mod transport;
#[cfg(test)]
mod tests;

pub(crate) use contracts::{
    ProviderRequestConnection, ProviderRequestInput, ProviderRequestPreparation,
    ProviderRequestResult, ProviderRequestTarget,
};
pub(crate) use registry::ProviderRequests;

use crate::commands::error::{blocking, operation_error, CommandError};
use crate::config_store::{ConfigStore, SavedProvider, StoreOperationError};
use registry::PreparedSource;
use std::time::Instant;

/// Reads one saved profile and returns its connection with the source revision
/// execute later re-checks. Every caller that can block on storage must call
/// this from the blocking pool: it scans, reads and parses provider files.
fn saved_connection(
    store: &ConfigStore,
    profile_id: &str,
) -> Result<(ProviderRequestConnection, String), CommandError> {
    let saved = store.resolve_saved_provider(profile_id).map_err(profile_error)?;
    let mut connection = ProviderRequestConnection::try_from(&saved.profile())?;
    let mut revision = saved.revision().to_string();
    if let SavedProvider::Codex { file, .. } = &saved {
        connection.default_model = Some(file.profile.resolve_model(&file.profile.default_model)
            .map_err(|message| CommandError::new("provider-request-model-invalid", message))?);
        if file.profile.connection.provider_type.as_deref() == Some("xai_oauth") {
            let account = crate::xai_auth::valid_token_for_profile(store.state_root(), profile_id)
                .map_err(|message| CommandError::new("xai-account-unavailable", message))?;
            // Account credentials use the provider-owned origin and models path.
            connection.connection.is_full_url = false;
            connection.connection.models_url = None;
            connection.api_key = account.access_token;
            connection.authentication = Some(asb_core::AuthenticationScheme::Bearer);
            revision = asb_switch::sha256_hex(&format!("{revision}:{}", account.account_id));
        }
    }
    Ok((connection, revision))
}

/// The one owner of "request target → connection" resolution. The saved
/// branch carries the file revision and managed account identity so execute
/// rejects a changed source before sending a request.
fn resolve(
    store: &ConfigStore,
    target: ProviderRequestTarget,
) -> Result<(ProviderRequestConnection, PreparedSource), CommandError> {
    match target {
        ProviderRequestTarget::Saved { profile_id } => {
            let (connection, revision) = saved_connection(store, &profile_id)?;
            Ok((
                connection,
                PreparedSource::Saved {
                    profile_id,
                    revision,
                },
            ))
        }
        ProviderRequestTarget::Draft { connection } => {
            if connection.app == asb_core::AppKind::Codex
                && connection.connection.provider_type.as_deref() == Some("xai_oauth") {
                return Err(CommandError::keyed("provider-request-saved-required", "errors.misc.providerRequestSavedRequired",
                    "xAI 托管请求需要已保存供应商的账号绑定，请保存后再测试"));
            }
            let source = PreparedSource::Draft(connection.clone());
            Ok((connection, source))
        }
    }
}

/// Rebuilds the connection a preparation token still stands for. The backend
/// is the only holder of the credential: drafts live in the registry, saved
/// profiles are re-read from storage inside the blocking pool.
fn resolve_prepared(
    store: &ConfigStore,
    source: PreparedSource,
) -> Result<ProviderRequestConnection, CommandError> {
    match source {
        PreparedSource::Draft(connection) => Ok(connection),
        PreparedSource::Saved {
            profile_id,
            revision,
        } => {
            let (connection, current_revision) = saved_connection(store, &profile_id)?;
            if current_revision != revision {
                return Err(profile_changed());
            }
            Ok(connection)
        }
    }
}

pub(crate) fn prepare(
    requests: &ProviderRequests,
    store: &ConfigStore,
    target: ProviderRequestTarget,
) -> Result<ProviderRequestPreparation, CommandError> {
    let (connection, source) = resolve(store, target)?;
    let connection = claude::resolve(store, connection, None)?;
    let endpoint = connection.endpoint(None)?;
    requests.issue(&connection, source, endpoint)
}

/// Models from the connection a preparation token stands for. The renderer
/// supplies only the opaque token, so no credential travels with the command;
/// the key travels only in the models request header. Storage reads and the
/// blocking provider call both run on the blocking pool.
pub(crate) async fn fetch_models(
    requests: ProviderRequests,
    store: ConfigStore,
    request_id: String,
) -> Result<Vec<crate::probe::ProviderModel>, CommandError> {
    let prepared = requests.inspect(&request_id)?;
    blocking(move || {
        let connection = resolve_prepared(&store, prepared.source)?;
        let connection = claude::resolve(&store, connection, prepared.claude_account.as_ref())?;
        if let Some(account) = &connection.claude_account {
            return crate::claude_auth::models::fetch_for_provider(
                account,
                connection.upstream_protocol,
                &connection.connection,
            )
            .map_err(|message| CommandError::new("models-fetch-failed", message));
        }
        crate::probe::fetch_models(
            &connection.base_url,
            &connection.api_key,
            connection.upstream_protocol,
            connection.authentication,
            &connection.connection,
        )
        .map_err(|message| CommandError {
            code: "models-fetch-failed",
            message,
            message_key: None,
            params: None,
        })
    })
    .await
}

pub(crate) async fn execute(
    requests: ProviderRequests,
    store: ConfigStore,
    input: ProviderRequestInput,
) -> Result<ProviderRequestResult, CommandError> {
    execute_with_client(requests, store, input, transport::client()?).await
}

async fn execute_with_client(
    requests: ProviderRequests,
    store: ConfigStore,
    input: ProviderRequestInput,
    client: reqwest::Client,
) -> Result<ProviderRequestResult, CommandError> {
    let model = input.model.trim().to_string();
    if model.is_empty() || model.chars().count() > 256 || model.chars().any(char::is_control) {
        return Err(CommandError::keyed(
            "provider-request-model-invalid",
            "errors.misc.providerRequestModelInvalid",
            "请填写有效的模型 ID（1–256 个字符，不含控制字符）",
        ));
    }
    let prepared = requests.inspect(&input.request_id)?;
    let started = Instant::now();
    let (task, active) = requests.start(&input.request_id, async move {
        let connection = blocking(move || {
            let connection = resolve_prepared(&store, prepared.source)?;
            claude::resolve(&store, connection, prepared.claude_account.as_ref())
        })
        .await?;
        let endpoint = connection.endpoint(Some(&model))?;
        transport::send(client, connection, endpoint, model, started).await
    })?;
    let result = registry::join(&task).await;
    if active.finish()? {
        return Ok(ProviderRequestResult::cancelled(started));
    }
    result.ok_or_else(interrupted)?.map_err(|_| interrupted())?
}

fn profile_error(error: StoreOperationError) -> CommandError {
    operation_error("provider-request-profile-unavailable", error)
}

fn profile_changed() -> CommandError {
    CommandError::keyed(
        "provider-request-profile-changed",
        "errors.misc.providerRequestProfileChanged",
        "供应商档案已变化，请重新准备并核对请求目标后再发送",
    )
}

fn interrupted() -> CommandError {
    CommandError::keyed(
        "provider-request-interrupted",
        "errors.misc.providerRequestInterrupted",
        "真实请求任务中断，请重试",
    )
}

