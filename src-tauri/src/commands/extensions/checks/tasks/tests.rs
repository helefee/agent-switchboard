use super::*;
use asb_core::{contracts::AppKind, extensions::contracts::{ExtensionTarget, McpCheckOutcome}};

fn result() -> McpCheckResult {
    McpCheckResult {
        definition_id: "fixture".into(), definition_revision: 1,
        target: ExtensionTarget::App { client: AppKind::Codex }, checked_at: "2026-10-13T00:00:00Z".into(),
        protocol_version: None, outcome: McpCheckOutcome::Cancelled, duration_ms: 1, truncated: false,
    }
}

#[test]
fn cancellation_sets_the_task_flag_and_terminal_status_is_consumed() {
    let (id, _) = start();
    let flag = tasks().lock().unwrap().get(&id).unwrap().cancel.clone();
    assert!(matches!(status(&id).unwrap(), CheckStatusDto::Running));
    assert!(cancel(&id));
    assert!(flag.load(Ordering::SeqCst));
    complete(&id, Ok(result()));
    assert!(matches!(status(&id).unwrap(), CheckStatusDto::Completed { .. }));
    assert!(status(&id).is_err());
    assert!(!cancel(&id));
}

#[test]
fn persistence_failure_is_terminal_instead_of_permanent_running() {
    let (id, _) = start();
    complete(&id, Err(CommandError::new("extension-store", "fixture failure")));
    assert_eq!(status(&id).err().unwrap().code, "extension-store");
    assert!(status(&id).is_err());
}
