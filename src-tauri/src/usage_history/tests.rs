use super::*;
use crate::local_state::LocalState;
use data::{HistoryImport, OfficialHistoryPoint, ProviderHistoryPoint};
use std::fs;

fn provider(index: u32) -> ProviderHistoryPoint {
    ProviderHistoryPoint { profile_id: "fixture-provider".into(), query_digest: "fixture-query".into(),
        at: (chrono::DateTime::parse_from_rfc3339("2020-01-01T00:00:00Z").unwrap()
            + chrono::Duration::minutes(index as i64)).to_rfc3339(),
        plan_name: None, unit: Some("USD".into()), remaining: Some(index as f64), used: None, total: None }
}
fn official(value: f64) -> OfficialHistoryPoint {
    OfficialHistoryPoint { at: "2026-10-16T01:00:00Z".into(), window_label: "primary".into(),
        used_percent: value, resets_at: None }
}

#[test]
fn migration_imports_every_old_point_once_then_retires_the_json_owner() {
    let temporary = tempfile::tempdir().unwrap();
    let state = LocalState::from_root(temporary.path().join("state"));
    fs::create_dir_all(state.root()).unwrap();
    let original = HistoryImport { providers: (0..1_500).map(provider).collect(), official: vec![official(12.0)] };
    let bytes = serde_json::to_vec(&original).unwrap();
    fs::write(state.root().join("usage-history.json"), &bytes).unwrap();
    initialize(&state).unwrap();
    initialize(&state).unwrap();
    let connection = store::read(&state).unwrap();
    let count: i64 = connection.query_row("SELECT COUNT(*) FROM provider_points", [], |row| row.get(0)).unwrap();
    assert_eq!(count, 1_500, "neither age nor point-count pruning is permitted");
    assert!(!state.root().join("usage-history.json").exists());
    let backup = fs::read_dir(state.root().join("repair-backups")).unwrap().next().unwrap().unwrap().path();
    assert_eq!(fs::read(backup).unwrap(), bytes);
    store::append_provider(&state, &[provider(1_500)]).unwrap();
    assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM provider_points", [], |row| row.get(0)).unwrap(), 1_501);
}

#[test]
fn account_changes_separate_the_current_series_without_deleting_history() {
    let temporary = tempfile::tempdir().unwrap();
    let state = LocalState::from_root(temporary.path().join("state"));
    initialize(&state).unwrap();
    store::append_official(&state, false, &[official(42.0)]).unwrap();
    store::append_official(&state, true, &[official(3.0)]).unwrap();
    let series = official_series(&state).unwrap();
    assert_eq!(series.len(), 1);
    assert_eq!(series[0].points[0].value, 3.0);
    let connection = store::read(&state).unwrap();
    assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM official_points", [], |row| row.get(0)).unwrap(), 2);
}

#[test]
fn invalid_original_history_is_preserved_and_no_partial_schema_commits() {
    let temporary = tempfile::tempdir().unwrap();
    let state = LocalState::from_root(temporary.path().join("state"));
    fs::create_dir_all(state.root()).unwrap();
    let original = state.root().join("usage-history.json");
    fs::write(&original, b"invalid original").unwrap();
    assert!(initialize(&state).is_err());
    assert_eq!(fs::read(&original).unwrap(), b"invalid original");
    let connection = rusqlite::Connection::open(store::path(&state)).unwrap();
    assert_eq!(connection.query_row::<i64,_,_>("PRAGMA user_version", [], |row| row.get(0)).unwrap(), 0);
}

#[test]
fn interrupted_import_archival_resumes_without_a_second_import() {
    let temporary = tempfile::tempdir().unwrap();
    let state = LocalState::from_root(temporary.path().join("state"));
    fs::create_dir_all(state.root()).unwrap();
    let bytes = serde_json::to_vec(&HistoryImport { providers: vec![provider(0)], official: vec![] }).unwrap();
    let original = state.root().join("usage-history.json");
    fs::write(&original, &bytes).unwrap();
    initialize(&state).unwrap();
    let backup = fs::read_dir(state.root().join("repair-backups")).unwrap().next().unwrap().unwrap().path();
    fs::rename(&backup, &original).unwrap();
    initialize(&state).unwrap();
    let connection = store::read(&state).unwrap();
    assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM provider_points", [], |row| row.get(0)).unwrap(), 1);
    assert!(backup.exists());
}

#[test]
fn failed_archival_keeps_history_quiescent_and_retry_does_not_duplicate_points() {
    let temporary = tempfile::tempdir().unwrap();
    let state = LocalState::from_root(temporary.path().join("state"));
    fs::create_dir_all(state.root()).unwrap();
    fs::write(state.root().join("usage-history.json"), serde_json::to_vec(&HistoryImport {
        providers: vec![provider(0)], official: vec![] }).unwrap()).unwrap();
    let blocked = state.root().join("repair-backups");
    fs::write(&blocked, b"blocking file").unwrap();
    assert!(initialize(&state).is_err());
    assert!(store::read(&state).is_err());
    assert!(store::append_provider(&state, &[provider(1)]).is_err());
    fs::remove_file(&blocked).unwrap();
    initialize(&state).unwrap();
    let connection = store::read(&state).unwrap();
    assert_eq!(connection.query_row::<i64,_,_>("SELECT COUNT(*) FROM provider_points", [], |row| row.get(0)).unwrap(), 1);
}

#[test]
fn an_open_chart_snapshot_does_not_block_recording_a_new_reading() {
    let temporary = tempfile::tempdir().unwrap();
    let state = LocalState::from_root(temporary.path().join("state"));
    initialize(&state).unwrap();
    let read = store::read(&state).unwrap();
    read.execute_batch("BEGIN DEFERRED").unwrap();
    let count = || read.query_row::<i64,_,_>("SELECT COUNT(*) FROM provider_points", [], |row| row.get(0)).unwrap();
    assert_eq!(count(), 0);
    store::append_provider(&state, &[provider(0)]).unwrap();
    assert_eq!(count(), 0, "the existing chart keeps its own consistent snapshot");
    read.execute_batch("COMMIT").unwrap();
    assert_eq!(count(), 1);
}

#[test]
fn missing_history_storage_is_an_error_and_never_an_empty_success() {
    let temporary = tempfile::tempdir().unwrap();
    let state = LocalState::from_root(temporary.path().join("state"));
    assert!(official_series(&state).is_err());
    initialize(&state).unwrap();
    assert!(official_series(&state).unwrap().is_empty());
}
