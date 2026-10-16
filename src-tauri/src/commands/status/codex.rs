//! Codex connection and account identity, independent of model and preferences.
use crate::{
    commands::error::{store_error, CommandError},
    local_state::LocalState,
};
use asb_core::AppKind;
fn error(message: String) -> CommandError {
    CommandError::new("codex-state-unavailable", message)
}
pub(super) fn account_matches(state: &LocalState, id: &str) -> Result<bool, CommandError> {
    crate::codex_auth::projection::matches_binding(state, id).map_err(error)
}
pub(crate) fn active_direct(
    state: &LocalState,
    text: &str,
) -> Result<Option<String>, CommandError> {
    let document = text
        .parse::<toml_edit::DocumentMut>()
        .map_err(|_| CommandError::keyed("codex-state-unavailable", "errors.cfg.codexConfigInvalid", "Codex 配置格式无效"))?;
    let pointer = document
        .get("model_catalog_json")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let name = std::path::Path::new(pointer)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("");
    let endpoint = asb_core::adapter::route_state(AppKind::Codex, text).base_url;
    if endpoint
        .as_deref()
        .is_none_or(asb_core::adapter::codex::is_gateway_base_url)
    {
        return Ok(None);
    }
    let target = state
        .target(AppKind::Codex)
        .map_err(error)?
        .with_file_name("auth.json");
    let auth = match std::fs::read_to_string(target) {
        Ok(text) => serde_json::from_str::<serde_json::Value>(&text).ok(),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => return Err(CommandError::keyed("codex-state-unavailable", "errors.cfg.codexDirectAuthUnreadable", "无法读取 Codex 直连认证状态")),
    };
    let key = auth
        .as_ref()
        .and_then(|v| v.get("OPENAI_API_KEY"))
        .and_then(|v| v.as_str());
    for record in state
        .configuration()
        .list_codex_providers()
        .map_err(store_error)?
    {
        let profile = record.profile;
        if profile.route_mode == asb_core::contracts::CodexRouteMode::Direct
            && endpoint.as_deref() == Some(profile.endpoint.0.as_str())
            && key == Some(profile.api_key.as_str())
            && name.starts_with(&format!("agent-switchboard-codex-{}-", profile.id))
            && name.ends_with(".json")
        {
            return Ok(Some(profile.id));
        }
    }
    Ok(None)
}
