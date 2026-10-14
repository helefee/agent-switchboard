use super::{ConfigStore, ConfigWriteRecord, StoreOperationError};
use asb_core::contracts::AppKind;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;

struct Temporary(PathBuf);
impl Drop for Temporary {
    fn drop(&mut self) { let _ = fs::remove_file(&self.0); }
}

impl ConfigStore {
    pub(super) fn rewrite_history(&self, app: AppKind,
        mut retain: impl FnMut(&ConfigWriteRecord) -> bool, append: Option<&ConfigWriteRecord>) -> Result<(), StoreOperationError> {
        self.ensure_layout()?;
        let target = self.history_path(app);
        let parent = target.parent().ok_or("配置历史目录无效")?;
        fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        let temporary = Temporary(parent.join(format!(".history-{}.tmp", uuid::Uuid::new_v4())));
        let mut file = File::create_new(&temporary.0).map_err(|error| error.to_string())?;
        {
            let mut writer = BufWriter::new(&mut file);
            writer.write_all(b"{\"records\":[").map_err(|error| error.to_string())?;
            let mut first = true;
            self.visit_history(app, |record| {
                if retain(&record) { write_record(&mut writer, &mut first, &record)?; }
                Ok(())
            })?;
            if let Some(record) = append { write_record(&mut writer, &mut first, record)?; }
            writer.write_all(b"]}\n").and_then(|()| writer.flush()).map_err(|error| error.to_string())?;
        }
        file.sync_all().map_err(|error| error.to_string())?;
        drop(file);
        fs::rename(&temporary.0, target).map_err(|error| error.to_string())?;
        Ok(())
    }
}

fn write_record(writer: &mut impl Write, first: &mut bool, record: &ConfigWriteRecord) -> Result<(), String> {
    if !*first { writer.write_all(b",").map_err(|error| error.to_string())?; }
    serde_json::to_writer(writer, record).map_err(|error| error.to_string())?;
    *first = false;
    Ok(())
}
