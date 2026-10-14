use super::*;
use asb_core::contracts::{ConfigWriteRecord, WriteOperation};
use std::io::Read;

fn record(id: &str) -> ConfigWriteRecord {
    ConfigWriteRecord { app: AppKind::Codex, profile_id: None, profile_name: None,
        content_hash: "a".repeat(64), backup_id: id.into(), at: "2020-01-01T00:00:00Z".into(),
        operation: WriteOperation::Restore }
}

#[test]
fn exports_a_complete_archive_before_reclaiming_and_retains_latest_recovery() {
    let temp = tempfile::tempdir().unwrap();
    let local = LocalState::from_root(temp.path().join("state"));
    local.configuration().record_config_write(record("old")).unwrap();
    local.configuration().record_config_write(record("latest")).unwrap();
    let preview = plan(&local, "2020-02-01", &mut std::io::sink()).unwrap();
    assert_eq!(preview.configuration[&AppKind::Codex].records, 1);
    let destination = temp.path().join("history.jsonl.gz");
    let result = execute(&local, &preview, &destination).unwrap();
    assert_eq!(result.removed, 1);
    assert!(result.warnings.is_empty());
    assert_eq!(local.configuration().load_history(AppKind::Codex).unwrap(), vec![record("latest")]);
    let mut archive = String::new();
    flate2::read::GzDecoder::new(File::open(destination).unwrap()).read_to_string(&mut archive).unwrap();
    assert!(archive.contains("agent-switchboard-history"));
    assert!(archive.contains("\"backupId\":\"old\""));
    assert!(!archive.contains("\"backupId\":\"latest\""));
}

#[test]
fn rejects_stale_previews_before_writing_an_archive_or_changing_history() {
    let temp = tempfile::tempdir().unwrap();
    let local = LocalState::from_root(temp.path().join("state"));
    local.configuration().record_config_write(record("first")).unwrap();
    local.configuration().record_config_write(record("second")).unwrap();
    let preview = plan(&local, "2020-02-01", &mut std::io::sink()).unwrap();
    local.configuration().record_config_write(record("third")).unwrap();
    let destination = temp.path().join("history.jsonl.gz");
    assert!(execute(&local, &preview, &destination).is_err());
    assert!(!destination.exists());
    assert_eq!(local.configuration().load_history(AppKind::Codex).unwrap().len(), 3);
    let future = (Utc::now().date_naive() + chrono::Duration::days(40)).to_string();
    assert!(cutoff_date(&future).is_err());
}

#[test]
fn repeated_recovery_backup_ids_protect_only_the_latest_write_fact() {
    let temp = tempfile::tempdir().unwrap();
    let local = LocalState::from_root(temp.path().join("state"));
    for _ in 0..3 { local.configuration().record_config_write(record("same-backup")).unwrap(); }
    let preview = plan(&local, "2020-02-01", &mut std::io::sink()).unwrap();
    assert_eq!(preview.configuration[&AppKind::Codex].records, 2);
    let result = execute(&local, &preview, &temp.path().join("history.jsonl.gz")).unwrap();
    assert_eq!(result.removed, 2);
    assert_eq!(local.configuration().load_history(AppKind::Codex).unwrap(), vec![record("same-backup")]);
}
