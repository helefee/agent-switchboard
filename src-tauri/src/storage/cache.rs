use std::sync::atomic::{AtomicBool, Ordering};
use tauri::AppHandle;

static CLEARING: AtomicBool = AtomicBool::new(false);
pub(super) fn is_clearing() -> bool { CLEARING.load(Ordering::Acquire) }

#[cfg(windows)]
mod windows_cache {
    use super::*;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use tauri::Manager;
    use tokio::sync::oneshot;
    use webview2_com::{ClearBrowsingDataCompletedHandler, Microsoft::Web::WebView2::Win32::{
        ICoreWebView2_13, ICoreWebView2Profile2, COREWEBVIEW2_BROWSING_DATA_KINDS_DISK_CACHE,
    }};
    use windows::core::Interface;

    struct Completion { sender: Mutex<Option<oneshot::Sender<Result<(), String>>>> }
    impl Completion {
        fn finish(&self, result: Result<(), String>) {
            let mut sender = self.sender.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(sender) = sender.take() {
                CLEARING.store(false, Ordering::Release);
                let action = crate::runtime_log::RuntimeLogAction::StorageCacheMaintained;
                if result.is_ok() { crate::runtime_log::record_success(action); }
                else { crate::runtime_log::record_failure(action, "storage-cache-maintenance-failed"); }
                let _ = sender.send(result);
            }
        }
    }
    impl Drop for Completion {
        fn drop(&mut self) {
            let sender = self.sender.get_mut().unwrap_or_else(|poisoned| poisoned.into_inner());
            if let Some(sender) = sender.take() {
                CLEARING.store(false, Ordering::Release);
                crate::runtime_log::record_failure(crate::runtime_log::RuntimeLogAction::StorageCacheMaintained,
                    "storage-cache-completion-lost");
                let _ = sender.send(Err("浏览器缓存维护回调已关闭，未确认清理完成".into()));
            }
        }
    }

    pub(crate) async fn clear_cache(app: &AppHandle) -> Result<(), String> {
        let window = app.get_webview_window("main").ok_or("主窗口尚未准备好，无法清理浏览器缓存")?;
        CLEARING.compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map_err(|_| "浏览器缓存正在清理，请等待完成")?;
        let (sender, receiver) = oneshot::channel();
        let completion = Arc::new(Completion { sender: Mutex::new(Some(sender)) });
        let dispatched = completion.clone();
        if let Err(error) = window.with_webview(move |view| {
            let profile = unsafe { view.controller().CoreWebView2()
                .and_then(|core| core.cast::<ICoreWebView2_13>())
                .and_then(|core| core.Profile())
                .and_then(|profile| profile.cast::<ICoreWebView2Profile2>()) };
            match profile {
                Ok(profile) => {
                    let callback = dispatched.clone();
                    let handler = ClearBrowsingDataCompletedHandler::create(Box::new(move |result| {
                        callback.finish(result.map_err(|error| format!("浏览器缓存清理失败：{error}")));
                        Ok(())
                    }));
                    if let Err(error) = unsafe { profile.ClearBrowsingData(COREWEBVIEW2_BROWSING_DATA_KINDS_DISK_CACHE, &handler) } {
                        dispatched.finish(Err(format!("无法启动浏览器缓存清理：{error}")));
                    }
                }
                Err(error) => dispatched.finish(Err(format!("当前 WebView2 无法提供磁盘缓存维护接口：{error}"))),
            }
        }) { completion.finish(Err(error.to_string())); }
        match tokio::time::timeout(Duration::from_secs(60), receiver).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err("浏览器缓存清理结果通道已关闭".into()),
            Err(_) => Err("浏览器缓存清理尚未完成，后台操作仍在继续，请稍后刷新容量".into()),
        }
    }

    pub(crate) fn spawn(app: AppHandle) {
        let spawned = std::thread::Builder::new().name("storage-cache-maintenance".into()).spawn(move || loop {
            match super::super::sizes::cache_bytes(&app.config().identifier) {
                Ok(bytes) if bytes > super::super::sizes::CACHE_BUDGET_BYTES && !is_clearing() => {
                    if let Err(error) = tauri::async_runtime::block_on(clear_cache(&app)) {
                        log::warn!("浏览器缓存自动维护失败：{error}");
                    }
                }
                Err(error) => {
                    crate::runtime_log::record_failure(crate::runtime_log::RuntimeLogAction::StorageCacheMaintained, "storage-cache-measurement-failed");
                    log::warn!("浏览器缓存容量统计失败：{error}");
                }
                _ => {}
            }
            std::thread::sleep(Duration::from_secs(300));
        });
        if let Err(error) = spawned { log::warn!("无法启动浏览器缓存维护线程：{error}"); }
    }
}

#[cfg(windows)]
pub(crate) use windows_cache::{clear_cache, spawn};

#[cfg(not(windows))]
pub(crate) async fn clear_cache(_app: &AppHandle) -> Result<(), String> {
    Err("当前平台不提供 WebView2 磁盘缓存维护".into())
}

#[cfg(not(windows))]
pub(crate) fn spawn(_app: AppHandle) {}
