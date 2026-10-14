use super::{error::{blocking, state, CommandError}, ConfigWriteGate};
use crate::storage::{self, ArchiveOutcome, HistoryArchivePlan, StorageUsage};
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

fn storage_error(detail: String) -> CommandError {
    CommandError::localized("storage-maintenance-failed", "errors.storage.operationFailed", detail.clone(),
        serde_json::json!({"detail":detail}))
}

#[tauri::command]
pub(crate) async fn storage_usage(app: AppHandle) -> Result<StorageUsage, CommandError> {
    let local = state(&app)?;
    let identifier = app.config().identifier.clone();
    blocking(move || storage::usage(&local, &identifier).map_err(storage_error)).await
}

#[tauri::command]
pub(crate) async fn clear_webview_cache(app: AppHandle) -> Result<(), CommandError> {
    storage::clear_cache(&app).await.map_err(storage_error)
}

#[tauri::command]
pub(crate) async fn preview_history_archive(app: AppHandle, cutoff: String) -> Result<HistoryArchivePlan, CommandError> {
    let local = state(&app)?;
    blocking(move || storage::archive::plan(&local, &cutoff, &mut std::io::sink()).map_err(storage_error)).await
}

#[tauri::command]
pub(crate) async fn archive_history(app: AppHandle, plan: HistoryArchivePlan) -> Result<Option<ArchiveOutcome>, CommandError> {
    let local = state(&app)?;
    let gate = app.state::<ConfigWriteGate>().inner().clone();
    let cache_root = crate::app_paths::local_data_directory(&app.config().identifier).map_err(storage_error)?;
    blocking(move || {
        let Some(destination) = app.dialog().file().add_filter("History archive", &["gz"])
            .set_file_name(format!("agent-switchboard-history-before-{}.jsonl.gz", plan.cutoff)).blocking_save_file()
        else { return Ok(None); };
        let target = destination.into_path().map_err(|error| storage_error(error.to_string()))?;
        if !target.file_name().and_then(|name| name.to_str()).is_some_and(|name| name.ends_with(".jsonl.gz")) {
            return Err(storage_error("历史归档文件名必须以 .jsonl.gz 结尾".into()));
        }
        let parent = target.parent().ok_or_else(|| storage_error("归档位置无效".into()))?
            .canonicalize().map_err(|error| storage_error(error.to_string()))?;
        for root in [local.root().parent().ok_or_else(|| storage_error("应用数据目录无效".into()))?.to_path_buf(), cache_root] {
            if root.exists() && parent.starts_with(root.canonicalize().map_err(|error| storage_error(error.to_string()))?) {
                return Err(storage_error("请选择应用数据目录之外的归档位置".into()));
            }
        }
        let _guard = gate.lock().map_err(storage_error)?;
        let result = storage::archive::execute(&local, &plan, &target);
        let action = crate::runtime_log::RuntimeLogAction::StorageHistoryArchived;
        match &result {
            Ok(outcome) if outcome.warnings.is_empty() => crate::runtime_log::record_success(action),
            Ok(_) => crate::runtime_log::record_failure(action, "storage-history-reclamation-incomplete"),
            Err(_) => crate::runtime_log::record_failure(action, "storage-history-archive-failed"),
        }
        result.map(Some).map_err(storage_error)
    }).await
}
