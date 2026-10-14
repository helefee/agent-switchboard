use super::{ConfigStore, ConfigWriteRecord};
use crate::storage::{ArchiveDigest, ArchiveWriter, ReclaimError, ReclaimOutcome};
use asb_core::contracts::AppKind;
use chrono::{DateTime, Utc};
use std::io::Write;

fn eligible(record: &ConfigWriteRecord, cutoff: DateTime<Utc>) -> bool {
    DateTime::parse_from_rfc3339(&record.at).is_ok_and(|at| at < cutoff)
}

impl ConfigStore {
    pub(crate) fn archive_history_digest(&self, app: AppKind, cutoff: DateTime<Utc>, output: &mut dyn Write)
        -> Result<ArchiveDigest, String> {
        let mut archive = ArchiveWriter::new(output, &format!("configuration:{app:?}:{cutoff}"));
        let mut previous = None;
        self.visit_history(app, |record| {
            if let Some(record) = previous.replace(record) {
                if eligible(&record, cutoff) {
                    archive.record("configurationWrite", &serde_json::json!({"app":app,"entry":record}))?;
                }
            }
            Ok(())
        }).map_err(|error| error.to_string())?;
        Ok(archive.finish())
    }

    pub(crate) fn prune_archived_history(&self, app: AppKind, cutoff: DateTime<Utc>, expected: &ArchiveDigest)
        -> Result<ReclaimOutcome, ReclaimError> {
        if self.archive_history_digest(app, cutoff, &mut std::io::sink())? != *expected {
            return Err(ReclaimError::Changed);
        }
        if expected.records == 0 { return Ok(ReclaimOutcome::default()); }
        let mut count = 0;
        self.visit_history(app, |_| { count += 1; Ok(()) }).map_err(|error| error.to_string())?;
        let mut position = 0;
        self.rewrite_history(app, |record| { position += 1; position == count || !eligible(record, cutoff) }, None)
            .map_err(|error| error.to_string())?;
        Ok(ReclaimOutcome { removed: expected.records, warning: None })
    }
}
