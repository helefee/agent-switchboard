//! Explicit MCP probes against the library definition in a selected target context.
use asb_core::extensions::contracts::{ExtensionPayload, ExtensionTarget};
use serde::Serialize;
use tauri::AppHandle;
use super::support::*;
use crate::commands::error::{state, CommandError};
use crate::extensions::checks;
use crate::extensions::secrets::{SecretBackend, SystemSecrets};

mod context;
mod tasks;
pub use tasks::CheckStatusDto;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckStartedDto { check_id: String }

#[tauri::command]
pub async fn check_mcp_connection(
    app: AppHandle, definition_id: String, definition_revision: u64,
    target: ExtensionTarget, confirm: bool,
) -> Result<CheckStartedDto, CommandError> {
    if !confirm {
        return Err(CommandError::keyed("check-not-confirmed", "errors.extops.checkRequiresConfirmation",
            "连接检测会运行服务命令或访问端点；需要明确确认"));
    }
    let state = state(&app)?;
    let store = extension_store(&state);
    let definition = store.get_definition(&definition_id).map_err(store_error)?
        .ok_or_else(|| CommandError::keyed("extension-not-found",
            "errors.extops.definitionNotFound", "扩展不存在或已被删除"))?;
    let ExtensionPayload::Mcp(mcp) = &definition.payload else {
        return Err(CommandError::keyed("extension-invalid", "errors.extops.checkOnlyForMcp",
            "连接检测只适用于 MCP 服务"));
    };
    let request = context::request_for(&store, &definition, mcp, target, definition_revision)?;
    let mcp = mcp.clone();
    let (check_id, deadline) = tasks::start();
    let thread_check_id = check_id.clone();
    let spawned = std::thread::Builder::new().name("mcp-check".into()).spawn(move || {
        let result = checks::run_check(&request, &mcp, &deadline, &|reference| SystemSecrets.get(reference).ok());
        let completed = store.save_check_if_current(&result).map(|_| result).map_err(store_error);
        tasks::complete(&thread_check_id, completed);
    });
    if let Err(error) = spawned {
        tasks::complete(&check_id, Err(CommandError::new("check-start-failed",
            format!("无法启动检测任务（{:?}）", error.kind()))));
    }
    Ok(CheckStartedDto { check_id })
}

#[tauri::command]
pub async fn get_mcp_check(check_id: String) -> Result<CheckStatusDto, CommandError> {
    tasks::status(&check_id)
}

#[tauri::command]
pub async fn cancel_mcp_check(check_id: String) -> Result<bool, CommandError> {
    Ok(tasks::cancel(&check_id))
}
