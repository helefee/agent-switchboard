use super::data::{normalized_timestamp, OfficialHistoryPoint, ProviderHistoryPoint};
use crate::local_state::LocalState;
use rusqlite::{params, Connection, OpenFlags};
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

static WRITE_GATE: Mutex<()> = Mutex::new(());
const SCHEMA: &str = "
CREATE TABLE provider_points (
 id INTEGER PRIMARY KEY, profile_id TEXT NOT NULL CHECK(length(profile_id)>0),
 query_digest TEXT NOT NULL CHECK(length(query_digest)>0),
 at TEXT NOT NULL CHECK(strftime('%Y-%m-%dT%H:%M:%fZ',julianday(at)) IS NOT NULL AND strftime('%Y-%m-%dT%H:%M:%fZ',julianday(at))=at), plan_name TEXT, unit TEXT,
 remaining REAL CHECK(remaining BETWEEN -1.7976931348623157e308 AND 1.7976931348623157e308),
 used REAL CHECK(used BETWEEN -1.7976931348623157e308 AND 1.7976931348623157e308),
 total REAL CHECK(total BETWEEN -1.7976931348623157e308 AND 1.7976931348623157e308),
 CHECK(remaining IS NOT NULL OR used IS NOT NULL OR total IS NOT NULL)
) STRICT;
CREATE INDEX provider_series ON provider_points(profile_id,query_digest,at,id);
CREATE TABLE official_points (
 id INTEGER PRIMARY KEY, generation INTEGER NOT NULL CHECK(generation>=0),
 at TEXT NOT NULL CHECK(strftime('%Y-%m-%dT%H:%M:%fZ',julianday(at)) IS NOT NULL AND strftime('%Y-%m-%dT%H:%M:%fZ',julianday(at))=at),
 window_label TEXT NOT NULL CHECK(length(trim(window_label))>0),
 used_percent REAL NOT NULL CHECK(used_percent BETWEEN 0 AND 100),
 resets_at TEXT CHECK(resets_at IS NULL OR (strftime('%Y-%m-%dT%H:%M:%fZ',julianday(resets_at)) IS NOT NULL AND strftime('%Y-%m-%dT%H:%M:%fZ',julianday(resets_at))=resets_at))
) STRICT;
CREATE INDEX official_series ON official_points(generation,window_label,at,id);
CREATE TABLE history_state (
 singleton INTEGER PRIMARY KEY CHECK(singleton=1), official_generation INTEGER NOT NULL CHECK(official_generation>=0),
 migration_ready INTEGER NOT NULL CHECK(migration_ready IN (0,1)),
 imported_json_sha256 TEXT CHECK(imported_json_sha256 IS NULL OR (length(imported_json_sha256)=64 AND imported_json_sha256 NOT GLOB '*[^a-f0-9]*')),
 initialization_error TEXT
) STRICT;
INSERT INTO history_state VALUES(1,0,0,NULL,NULL);
PRAGMA user_version=1;";

pub(super) fn path(state: &LocalState) -> PathBuf { state.root().join("usage-history.sqlite3") }

fn connection(state: &LocalState, write: bool) -> Result<Connection, String> {
    let flags = if write { OpenFlags::SQLITE_OPEN_READ_WRITE } else { OpenFlags::SQLITE_OPEN_READ_ONLY };
    let connection = Connection::open_with_flags(path(state), flags).map_err(|error| error.to_string())?;
    connection.busy_timeout(Duration::from_secs(2)).map_err(|error| error.to_string())?;
    if write { connection.pragma_update(None, "synchronous", "FULL").map_err(|error| error.to_string())?; }
    let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0)).map_err(|error| error.to_string())?;
    if version != 1 { return Err("用量历史尚未完成初始化或版本不受支持，原数据已保留；请检查迁移诊断后重启重试".into()); }
    let mode: String = connection.pragma_query_value(None, "journal_mode", |row| row.get(0)).map_err(|error| error.to_string())?;
    if !mode.eq_ignore_ascii_case("wal") { return Err("用量历史存储尚未启用并发读写模式，原数据已保留".into()); }
    let (ready, issue): (i64, Option<String>) = connection.query_row(
        "SELECT migration_ready,initialization_error FROM history_state WHERE singleton=1",
        [], |row| Ok((row.get(0)?, row.get(1)?))).map_err(|error| error.to_string())?;
    if ready != 1 { return Err(issue.unwrap_or_else(|| "用量历史迁移尚未完成，原件已保留；重启会重试迁移".into())); }
    Ok(connection)
}

pub(crate) fn initialize(state: &LocalState) -> Result<(), String> {
    let _guard = WRITE_GATE.lock().map_err(|_| "用量历史写入锁不可用")?;
    std::fs::create_dir_all(state.root()).map_err(|error| error.to_string())?;
    let mut connection = Connection::open(path(state)).map_err(|error| error.to_string())?;
    connection.busy_timeout(Duration::from_secs(2)).map_err(|error| error.to_string())?;
    connection.pragma_update(None, "synchronous", "FULL").map_err(|error| error.to_string())?;
    let version: i64 = connection.query_row("PRAGMA user_version", [], |row| row.get(0)).map_err(|error| error.to_string())?;
    if version == 0 {
        let tables: i64 = connection.query_row("SELECT COUNT(*) FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%'", [], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        if tables != 0 { return Err("用量历史数据库结构未知，已保留原件且未修改".into()); }
        let transaction = connection.transaction().map_err(|error| error.to_string())?;
        transaction.execute_batch(SCHEMA).map_err(|error| error.to_string())?;
        super::migration::import_json(state, &transaction)?;
        transaction.commit().map_err(|error| error.to_string())?;
    } else if version != 1 { return Err("用量历史数据库版本不受支持，原数据已保留".into()); }
    let ready: i64 = connection.query_row("SELECT migration_ready FROM history_state WHERE singleton=1", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if !matches!(ready, 0 | 1) { return Err("用量历史初始化状态无效，原件已保留".into()); }
    let changed = connection.execute("UPDATE history_state SET migration_ready=0 WHERE singleton=1", []).map_err(|error| error.to_string())?;
    if changed != 1 { return Err("用量历史数据库缺少当前初始化状态，原数据已保留".into()); }
    connection.pragma_update(None, "journal_mode", "WAL").map_err(|error| error.to_string())?;
    let mode: String = connection.pragma_query_value(None, "journal_mode", |row| row.get(0)).map_err(|error| error.to_string())?;
    if !mode.eq_ignore_ascii_case("wal") { return Err("无法启用用量历史的并发读写模式，原数据已保留".into()); }
    if let Err(issue) = super::migration::archive_imported_json(state, &connection, ready == 1) {
        connection.execute("UPDATE history_state SET initialization_error=?1 WHERE singleton=1", [&issue])
            .map_err(|error| format!("{issue}；保存诊断失败: {error}"))?;
        return Err(issue);
    }
    connection.execute_batch("BEGIN IMMEDIATE; UPDATE history_state SET migration_ready=1,initialization_error=NULL WHERE singleton=1; COMMIT;").map_err(|error| error.to_string())
}

pub(super) fn insert_provider(connection: &Connection, point: &ProviderHistoryPoint) -> Result<(), String> {
    point.validate()?;
    connection.execute("INSERT INTO provider_points(profile_id,query_digest,at,plan_name,unit,remaining,used,total) \
        VALUES(?1,?2,?3,?4,?5,?6,?7,?8)", params![point.profile_id,point.query_digest,normalized_timestamp(&point.at)?,
        point.plan_name,point.unit,point.remaining,point.used,point.total]).map_err(|error| error.to_string())?;
    Ok(())
}
pub(super) fn insert_official(connection: &Connection, generation: i64, point: &OfficialHistoryPoint) -> Result<(), String> {
    point.validate()?;
    connection.execute("INSERT INTO official_points(generation,at,window_label,used_percent,resets_at) VALUES(?1,?2,?3,?4,?5)",
        params![generation,normalized_timestamp(&point.at)?,point.window_label,point.used_percent,
            point.resets_at.as_deref().map(normalized_timestamp).transpose()?]).map_err(|error| error.to_string())?;
    Ok(())
}

pub(super) fn append_provider(state: &LocalState, points: &[ProviderHistoryPoint]) -> Result<(), String> {
    let _guard = WRITE_GATE.lock().map_err(|_| "用量历史写入锁不可用")?;
    let mut connection = connection(state, true)?;
    let transaction = connection.transaction().map_err(|error| error.to_string())?;
    for point in points { insert_provider(&transaction, point)?; }
    transaction.commit().map_err(|error| error.to_string())
}
pub(super) fn append_official(state: &LocalState, reset: bool, points: &[OfficialHistoryPoint]) -> Result<(), String> {
    let _guard = WRITE_GATE.lock().map_err(|_| "用量历史写入锁不可用")?;
    let mut connection = connection(state, true)?;
    let transaction = connection.transaction().map_err(|error| error.to_string())?;
    if reset { transaction.execute("UPDATE history_state SET official_generation=official_generation+1 WHERE singleton=1", [])
        .map_err(|error| error.to_string())?; }
    let generation = generation(&transaction)?;
    for point in points { insert_official(&transaction, generation, point)?; }
    transaction.commit().map_err(|error| error.to_string())
}
pub(super) fn read(state: &LocalState) -> Result<Connection, String> {
    if !path(state).try_exists().map_err(|error| error.to_string())? {
        return Err("用量历史存储尚未完成初始化，不能将读取失败显示为空历史；请检查启动诊断后重试".into());
    }
    connection(state, false)
}
pub(super) fn generation(connection: &Connection) -> Result<i64, String> {
    connection.query_row("SELECT official_generation FROM history_state WHERE singleton=1", [], |row| row.get(0))
        .map_err(|error| error.to_string())
}
