use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};
use asb_core::extensions::contracts::McpCheckResult;
use serde::Serialize;
use crate::commands::error::CommandError;
use crate::extensions::checks::{CheckDeadline, TOTAL_DEADLINE};

type Completion = Result<McpCheckResult, CommandError>;
struct Task {
    cancel: Arc<AtomicBool>,
    completion: Option<Completion>,
    expires_at: Instant,
}
static TASKS: OnceLock<Mutex<BTreeMap<String, Task>>> = OnceLock::new();

#[derive(Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum CheckStatusDto { Running, Completed { result: McpCheckResult } }

fn tasks() -> &'static Mutex<BTreeMap<String, Task>> {
    TASKS.get_or_init(|| Mutex::new(BTreeMap::new()))
}

fn prune(tasks: &mut BTreeMap<String, Task>) {
    let now = Instant::now();
    tasks.retain(|_, task| {
        if task.expires_at > now { return true; }
        task.cancel.store(true, Ordering::SeqCst);
        false
    });
}

pub(super) fn start() -> (String, CheckDeadline) {
    let mut tasks = tasks().lock().expect("check tasks");
    prune(&mut tasks);
    let id = super::new_id("check");
    let cancel = Arc::new(AtomicBool::new(false));
    let deadline = CheckDeadline::new(cancel.clone());
    tasks.insert(id.clone(), Task {
        cancel, completion: None, expires_at: Instant::now() + TOTAL_DEADLINE + Duration::from_secs(120),
    });
    (id, deadline)
}

pub(super) fn complete(id: &str, result: Completion) {
    let mut tasks = tasks().lock().expect("check tasks");
    if let Some(task) = tasks.get_mut(id) {
        task.completion = Some(result);
        task.expires_at = Instant::now() + Duration::from_secs(120);
    }
}

pub(super) fn status(id: &str) -> Result<CheckStatusDto, CommandError> {
    let mut tasks = tasks().lock().expect("check tasks");
    prune(&mut tasks);
    let task = tasks.get_mut(id).ok_or_else(|| CommandError::keyed(
        "check-not-found", "errors.extops.checkNotFound", "检测任务不存在或已结束",
    ))?;
    if let Some(result) = task.completion.take() {
        tasks.remove(id);
        result.map(|result| CheckStatusDto::Completed { result })
    } else {
        Ok(CheckStatusDto::Running)
    }
}

pub(super) fn cancel(id: &str) -> bool {
    let mut tasks = tasks().lock().expect("check tasks");
    prune(&mut tasks);
    if let Some(task) = tasks.get(id).filter(|task| task.completion.is_none()) {
        task.cancel.store(true, Ordering::SeqCst);
        true
    } else { false }
}

#[cfg(test)]
mod tests;
