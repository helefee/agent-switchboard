//! The renderer's only door to the outbound egress policy. Read-only
//! otherwise: gateway upstreams, probes and quota reads all consume the
//! `outbound_proxy` snapshot and never accept per-request proxy input.

use super::error::{blocking, observe, state, CommandError};
use crate::outbound_proxy::{self, OutboundProxyRepairReport, OutboundProxySettings};
use crate::runtime_log::RuntimeLogAction;

pub(super) fn settings_error(error: crate::outbound_proxy::OutboundProxyError) -> CommandError {
    let (key, params) = error.parts();
    let code = if error.is_io() { "outbound-proxy-unavailable" } else { "outbound-proxy-invalid" };
    CommandError::localized(code, key, error.to_string(), params)
}

#[tauri::command]
pub async fn get_outbound_proxy_settings(
    app: tauri::AppHandle,
) -> Result<OutboundProxySettings, CommandError> {
    let state = state(&app)?;
    blocking(move || outbound_proxy::read(state.root()).map_err(settings_error)).await
}

#[tauri::command]
pub async fn set_outbound_proxy_settings(
    app: tauri::AppHandle,
    settings: OutboundProxySettings,
    expected_revision: String,
) -> Result<OutboundProxySettings, CommandError> {
    let state = state(&app)?;
    let saved = observe(RuntimeLogAction::OutboundProxySaved, async move {
        blocking(move || {
            outbound_proxy::save(state.root(), settings, &expected_revision).map_err(settings_error)
        })
        .await
    })
    .await?;
    Ok(saved)
}

/// Uses the same recheck-and-backup repair as the global data-repair entry.
#[tauri::command]
pub async fn repair_outbound_proxy_settings(
    app: tauri::AppHandle,
) -> Result<OutboundProxyRepairReport, CommandError> {
    let state = state(&app)?;
    observe(RuntimeLogAction::OutboundProxyRepaired, async move {
        blocking(move || outbound_proxy::repair(state.root()).map_err(settings_error)).await
    })
    .await
}
