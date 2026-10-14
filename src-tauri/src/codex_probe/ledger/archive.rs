use super::{db_error, map_batch, map_run, ProbeLedger, BATCH_COLUMNS};
use crate::storage::{sqlite, ArchiveDigest, ArchiveWriter, ReclaimError, ReclaimOutcome};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use std::io::Write;

const ELIGIBLE: &str = "status!='running' AND julianday(started_at)<julianday(?1)";

fn scan(connection: &Connection, cutoff: DateTime<Utc>, output: &mut dyn Write) -> Result<ArchiveDigest, String> {
    let mut archive = ArchiveWriter::new(output, &format!("probeHistory:{cutoff}"));
    let mut statement = connection.prepare(&format!("SELECT {BATCH_COLUMNS} FROM probe_batches NOT INDEXED WHERE {ELIGIBLE} ORDER BY rowid")).map_err(db_error)?;
    let batches = statement.query_map([cutoff.to_rfc3339()], map_batch).map_err(db_error)?;
    let mut runs = connection.prepare(
        "SELECT seq,status,session_id,final_answer,reported_model,duration_ms,reasoning_tokens,total_tokens,execution_error,usage_error
         FROM probe_runs INDEXED BY probe_runs_batch WHERE batch_id=?1 ORDER BY seq"
    ).map_err(db_error)?;
    for batch in batches {
        let batch = batch.map_err(db_error)?;
        archive.record("probeBatch", &batch)?;
        for run in runs.query_map([&batch.id], map_run).map_err(db_error)? {
            archive.record("probeRun", &serde_json::json!({"batchId":batch.id,"entry":run.map_err(db_error)?}))?;
        }
    }
    Ok(archive.finish())
}

impl ProbeLedger {
    pub(crate) fn archive_digest(&self, cutoff: DateTime<Utc>, output: &mut dyn Write) -> Result<ArchiveDigest, String> {
        let Some(connection) = self.open_read()? else {
            return Ok(ArchiveWriter::new(output, &format!("probeHistory:{cutoff}")).finish());
        };
        scan(&connection, cutoff, output)
    }

    pub(crate) fn prune_archived(&self, cutoff: DateTime<Utc>, expected: &ArchiveDigest) -> Result<ReclaimOutcome, ReclaimError> {
        if expected.records == 0 { return Ok(ReclaimOutcome::default()); }
        let mut connection = self.open_write()?;
        let transaction = connection.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(db_error)?;
        if scan(&transaction, cutoff, &mut std::io::sink())? != *expected {
            return Err(ReclaimError::Changed);
        }
        let mut removed = transaction.execute(&format!(
            "DELETE FROM probe_runs WHERE batch_id IN (SELECT id FROM probe_batches WHERE {ELIGIBLE})"
        ), [cutoff.to_rfc3339()]).map_err(db_error)?;
        removed += transaction.execute(&format!("DELETE FROM probe_batches WHERE {ELIGIBLE}"), [cutoff.to_rfc3339()]).map_err(db_error)?;
        transaction.commit().map_err(db_error)?;
        Ok(ReclaimOutcome { removed: removed as u64,
            warning: sqlite::compact(&connection).err() })
    }
}
