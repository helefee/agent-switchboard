//! Global application-owned data repair; each domain keeps its own validator
//! and recovery material. Partial success is returned instead of concealed.
use super::error::{blocking, state, CommandError};
use crate::runtime_log::RuntimeLogAction;
use serde::Serialize;
use tauri::Manager;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApplicationDataRepairReport {
    repaired_files: Vec<String>,
    backup_paths: Vec<String>,
    issues: Vec<CommandError>,
}

#[tauri::command]
pub async fn repair_application_data(
    app: tauri::AppHandle,
) -> Result<ApplicationDataRepairReport, CommandError> {
    let local = state(&app)?;
    let gate = app.state::<super::ConfigWriteGate>().inner().clone();
    let refresh_app = app.clone();
    let result = blocking(move || {
        let _guard = gate.lock().map_err(|error| CommandError::new("config-write-gate-unavailable", error))?;
        let mut report = ApplicationDataRepairReport {
            repaired_files: Vec::new(), backup_paths: Vec::new(), issues: Vec::new(),
        };
        match local.configuration().repair() {
            Ok(provider) => {
                report.repaired_files.extend(provider.repaired_files.into_iter()
                    .map(|path| format!("configuration/{path}")));
                report.backup_paths.extend(provider.backup_path);
            }
            Err(error) => report.issues.push(CommandError::localized(
                "profile-store-repair-failed", "errors.sw.profileStoreRepairFailed",
                format!("供应商存储无法安全修复：{error}"), serde_json::json!({ "detail": error }),
            )),
        }
        match crate::outbound_proxy::repair(local.root()) {
            Ok(proxy) => {
                if let Some(path) = proxy.backup_path {
                    report.repaired_files.push("outbound-proxy.json".into());
                    report.backup_paths.push(path);
                }
            }
            Err(error) => report.issues.push(super::outbound_proxy_settings::settings_error(error)),
        }
        Ok(report)
    }).await;
    match &result {
        Ok(report) => match report.issues.first() {
            Some(issue) => crate::runtime_log::record_failure(RuntimeLogAction::ApplicationDataRepaired, issue.code),
            None => crate::runtime_log::record_success(RuntimeLogAction::ApplicationDataRepaired),
        },
        Err(error) => crate::runtime_log::record_failure(RuntimeLogAction::ApplicationDataRepaired, error.code),
    }
    crate::tray::refresh(&refresh_app);
    result
}
