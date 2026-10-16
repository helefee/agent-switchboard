use rusqlite::{Connection, OptionalExtension, TransactionBehavior};
use std::path::{Path, PathBuf};
use std::time::Duration;

pub(super) const SCHEMA_V1: &str = "CREATE TABLE bookmarks (
    id TEXT PRIMARY KEY NOT NULL,
    app TEXT NOT NULL CHECK(app IN ('codex', 'claude')),
    session_id TEXT NOT NULL,
    session_title TEXT NOT NULL,
    project_dir TEXT,
    message_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant')),
    content TEXT NOT NULL CHECK(length(content) > 0),
    at TEXT,
    saved_at TEXT NOT NULL,
    UNIQUE(app, session_id, message_id)
) STRICT";
pub(super) const SCHEMA: &str = "CREATE TABLE bookmarks (
    id TEXT PRIMARY KEY NOT NULL,
    app TEXT NOT NULL CHECK(app IN ('codex', 'claude')),
    session_id TEXT NOT NULL,
    session_title TEXT NOT NULL,
    project_dir TEXT,
    message_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('user', 'assistant')),
    content TEXT NOT NULL CHECK(length(CAST(content AS BLOB)) > 0),
    images TEXT NOT NULL CHECK(json_valid(images) AND json_type(images)='array'),
    at TEXT,
    saved_at TEXT NOT NULL,
    UNIQUE(app, session_id, message_id)
) STRICT";
pub(super) const COLUMNS: &str = "id, app, session_id, session_title, project_dir, message_id, role, content, at, saved_at";
static SCHEMA_GATE: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(super) fn database_path(root: &Path) -> PathBuf {
    super::super::layout::directory(root).join("collections.sqlite3")
}

pub(super) fn open_store(root: &Path) -> Result<Connection, String> {
    // The desktop has one process; serialize existence checks through schema
    // commit so another command cannot mistake a new file for a damaged DB.
    let _guard = SCHEMA_GATE.lock().map_err(|_| "收藏库初始化锁不可用")?;
    let path = database_path(root);
    let existed = path.exists();
    std::fs::create_dir_all(path.parent().ok_or("收藏目录无效")?)
        .map_err(|error| format!("无法创建收藏目录: {error}"))?;
    let mut connection = Connection::open(&path).map_err(database_error)?;
    connection.set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_LENGTH, (super::super::records::MAX_RECORD_BYTES + 65_536) as i32);
    connection.busy_timeout(Duration::from_secs(5)).map_err(database_error)?;
    let transaction = connection.transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(database_error)?;
    let version: i64 = transaction.query_row("PRAGMA user_version", [], |row| row.get(0))
        .map_err(database_error)?;
    let tables: i64 = transaction.query_row(
        "SELECT count(*) FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'",
        [], |row| row.get(0),
    ).map_err(database_error)?;
    if !existed && version == 0 && tables == 0 {
        transaction.execute_batch(SCHEMA).map_err(database_error)?;
        transaction.pragma_update(None, "user_version", 2).map_err(database_error)?;
    } else {
        let schema: Option<String> = transaction.query_row(
            "SELECT sql FROM sqlite_master WHERE type = 'table' AND name = 'bookmarks'",
            [], |row| row.get(0),
        ).optional().map_err(database_error)?;
        if version == 1 && tables == 1 && schema.as_deref() == Some(SCHEMA_V1) {
            transaction.execute_batch("ALTER TABLE bookmarks RENAME TO bookmarks_previous").map_err(database_error)?;
            transaction.execute_batch(SCHEMA).map_err(database_error)?;
            transaction.execute_batch(&format!("INSERT INTO bookmarks ({COLUMNS},images) SELECT {COLUMNS},'[]' FROM bookmarks_previous; DROP TABLE bookmarks_previous;"))
                .map_err(database_error)?;
            transaction.pragma_update(None, "user_version", 2).map_err(database_error)?;
        } else if version != 2 || tables != 1 || schema.as_deref() != Some(SCHEMA) {
            return Err(format!("收藏库结构无效，未修改原文件：{}", path.display()));
        }
    }
    transaction.execute_batch("CREATE INDEX IF NOT EXISTS bookmarks_saved_at ON bookmarks(saved_at DESC, id)")
        .map_err(database_error)?;
    transaction.commit().map_err(database_error)?;
    Ok(connection)
}

pub(super) fn database_error(error: rusqlite::Error) -> String {
    format!("收藏库操作失败: {error}")
}
