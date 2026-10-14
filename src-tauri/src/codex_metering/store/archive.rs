use super::{db_error, CodexRequestLedger, CodexRequestRecord};
use crate::storage::{sqlite, ArchiveDigest, ArchiveWriter, ReclaimError, ReclaimOutcome};
use chrono::{DateTime, Utc};
use rusqlite::Connection;
use std::io::Write;

fn scan(connection: &Connection, cutoff: DateTime<Utc>, output: &mut dyn Write) -> Result<ArchiveDigest, String> {
    let mut archive = ArchiveWriter::new(output, &format!("codexRequests:{cutoff}"));
    let mut statement = connection.prepare("SELECT payload FROM codex_requests NOT INDEXED WHERE at_ms<?1 ORDER BY rowid").map_err(db_error)?;
    let mut rows = statement.query([cutoff.timestamp_millis()]).map_err(db_error)?;
    while let Some(row) = rows.next().map_err(db_error)? {
        let payload: String = row.get(0).map_err(db_error)?;
        let record: CodexRequestRecord = serde_json::from_str(&payload).map_err(|error| error.to_string())?;
        record.validate()?;
        archive.record("codexRequest", &record)?;
    }
    Ok(archive.finish())
}

impl CodexRequestLedger {
    pub(crate) fn archive_digest(&self, cutoff: DateTime<Utc>, output: &mut dyn Write) -> Result<ArchiveDigest, String> {
        let Some(connection) = self.open_read()? else {
            return Ok(ArchiveWriter::new(output, &format!("codexRequests:{cutoff}")).finish());
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
        let removed = transaction.execute("DELETE FROM codex_requests WHERE at_ms<?1", [cutoff.timestamp_millis()]).map_err(db_error)?;
        transaction.commit().map_err(db_error)?;
        Ok(ReclaimOutcome { removed: removed as u64,
            warning: sqlite::compact(&connection).err() })
    }
}

#[cfg(test)]
mod tests;
