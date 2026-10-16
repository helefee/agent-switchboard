use super::{data::HistoryImport, store};
use crate::local_state::LocalState;
use rusqlite::Connection;

fn original(state: &LocalState) -> std::path::PathBuf { state.root().join("usage-history.json") }

pub(super) fn import_json(state: &LocalState, connection: &Connection) -> Result<(), String> {
    let bytes = match std::fs::read(original(state)) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("原用量历史不可读，已保留: {error}")),
    };
    let history: HistoryImport = serde_json::from_slice(&bytes).map_err(|error| format!("原用量历史格式无效，已保留: {error}"))?;
    for point in history.providers { store::insert_provider(connection, &point)?; }
    for point in history.official { store::insert_official(connection, 0, &point)?; }
    let digest = asb_switch::sha256_hex(&String::from_utf8(bytes).map_err(|error| error.to_string())?);
    connection.execute("UPDATE history_state SET imported_json_sha256=?1 WHERE singleton=1", [&digest])
        .map_err(|error| error.to_string())?;
    Ok(())
}

pub(super) fn archive_imported_json(state: &LocalState, connection: &Connection, completed: bool) -> Result<(), String> {
    let path = original(state);
    let digest: Option<String> = connection.query_row("SELECT imported_json_sha256 FROM history_state WHERE singleton=1", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if !path.try_exists().map_err(|error| error.to_string())? {
        if completed { return Ok(()); }
        if let Some(expected) = digest {
            let backup = state.root().join("repair-backups").join(format!("usage-history-before-sqlite-{expected}.json"));
            let text = std::fs::read_to_string(&backup).map_err(|error| format!("历史已导入，但未找到完整原件备份，数据库已保留: {error}"))?;
            if asb_switch::sha256_hex(&text) != expected { return Err("历史原件备份摘要不一致，数据库和备份均已保留".into()); }
            crate::storage::publish::sync_parent(&backup)?;
            crate::storage::publish::sync_parent(&path)?;
        }
        return Ok(());
    }
    let expected = digest.ok_or("用量历史数据库已存在，但原 JSON 尚未导入；已保留两份材料并停止写入")?;
    let text = std::fs::read_to_string(&path).map_err(|error| error.to_string())?;
    if asb_switch::sha256_hex(&text) != expected { return Err("用量历史迁移期间原 JSON 已改变，原件已保留".into()); }
    let directory = state.root().join("repair-backups");
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let target = directory.join(format!("usage-history-before-sqlite-{expected}.json"));
    if target.exists() { return Err(format!("用量历史迁移备份已存在，原件已保留: {}", target.display())); }
    crate::storage::publish::move_new(&path, &target).map_err(|error| format!("历史已导入，但原件归档尚未确认: {error}"))
}
