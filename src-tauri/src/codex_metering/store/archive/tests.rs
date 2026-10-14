use super::*;
use crate::codex_metering::{CodexBilling, CodexUsageOrigin};

fn record(at_ms: u64) -> CodexRequestRecord {
    CodexRequestRecord { id: uuid::Uuid::new_v4().to_string(), origin: CodexUsageOrigin::Proxy,
        thread_id: None, at_ms, billable: true, profile_id: None, route_revision: None,
        upstream_protocol: None, request_model: None, mapped_model: None, response_model: None,
        status: Some(200), duration_ms: 1, first_byte_latency_ms: None, first_token_latency_ms: None,
        input_tokens: Some(10), output_tokens: Some(1), cache_read_tokens: Some(0),
        cache_creation_tokens: None, reasoning_tokens: None, billing: CodexBilling::default(),
        cost: None, pricing_error: None, attempts: Vec::new() }
}

#[test]
fn request_archival_retains_the_cutoff_and_exports_each_removed_payload() {
    let temp = tempfile::tempdir().unwrap();
    let ledger = CodexRequestLedger::new(temp.path());
    let cutoff = DateTime::parse_from_rfc3339("2020-02-01T00:00:00Z").unwrap().with_timezone(&Utc);
    let old = record(cutoff.timestamp_millis() as u64 - 1);
    let protected = record(cutoff.timestamp_millis() as u64);
    ledger.append(&old).unwrap(); ledger.append(&protected).unwrap();
    let mut output = Vec::new();
    let preview = ledger.archive_digest(cutoff, &mut output).unwrap();
    assert_eq!(preview.records, 1);
    let exported: serde_json::Value = serde_json::from_slice(&output).unwrap();
    assert_eq!(exported["record"]["id"], old.id);
    assert_eq!(ledger.prune_archived(cutoff, &preview).unwrap().removed, 1);
    let connection = ledger.open_read().unwrap().unwrap();
    let ids = connection.prepare("SELECT id FROM codex_requests").unwrap()
        .query_map([], |row| row.get::<_, String>(0)).unwrap().collect::<Result<Vec<_>, _>>().unwrap();
    assert_eq!(ids, vec![protected.id]);
}

#[test]
fn old_requests_added_after_export_invalidate_reclamation() {
    let temp = tempfile::tempdir().unwrap();
    let ledger = CodexRequestLedger::new(temp.path());
    let cutoff = DateTime::parse_from_rfc3339("2020-02-01T00:00:00Z").unwrap().with_timezone(&Utc);
    ledger.append(&record(1)).unwrap();
    let preview = ledger.archive_digest(cutoff, &mut std::io::sink()).unwrap();
    ledger.append(&record(2)).unwrap();
    assert!(matches!(ledger.prune_archived(cutoff, &preview), Err(ReclaimError::Changed)));
    assert_eq!(ledger.archive_digest(cutoff, &mut std::io::sink()).unwrap().records, 2);
}
