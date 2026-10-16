use super::{native, parser, records, SessionMessage, SessionSource};
use chrono::{TimeZone, Utc};
use rusqlite::{Connection, params};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::ops::ControlFlow;
use std::path::PathBuf;

enum Location {
    Native { turn: String, item: String, realtime: bool },
    Jsonl { path: PathBuf, offset: u64, digest: String },
}
struct Entry { id: String, at: Option<String>, location: Location }

pub(super) fn visit(
    source: &SessionSource,
    mut visitor: impl FnMut(SessionMessage) -> Result<ControlFlow<()>, String>,
) -> Result<Vec<String>, String> {
    let history = source.native_history.as_ref().ok_or("缺少原生历史位置")?;
    let snapshot = native::open(history).and_then(|connection| {
        connection.execute_batch("BEGIN DEFERRED").map_err(|error| error.to_string())?;
        let index = native_index(&connection, &source.meta.session_id)?;
        Ok((connection, index))
    });
    let (connection, (mut entries, mut issues)) = match snapshot {
        Ok(snapshot) => snapshot,
        Err(error) => {
            let mut issues = vec![error];
            issues.extend(super::jsonl::visit(&source.paths, &mut visitor)?);
            return Ok(issues);
        }
    };
    if entries.is_empty() {
        if source.paths.is_empty() { issues.push("原生历史保留了会话身份，但未保留可读取的消息正文".into()); }
        else { issues.extend(super::jsonl::visit(&source.paths, &mut visitor)?); }
        return Ok(issues);
    }
    append_protocol(source, &mut entries, &mut issues);
    entries.sort_by(|left, right| left.at.cmp(&right.at));
    for entry in entries {
        let mut message = match load(&connection, &source.meta.session_id, &entry) {
            Ok(message) => message,
            Err(error) => { issues.push(error); continue; }
        };
        message.id = entry.id;
        message.at = entry.at;
        if visitor(message)?.is_break() { break; }
    }
    Ok(issues)
}

fn append_protocol(source: &SessionSource, entries: &mut Vec<Entry>, issues: &mut Vec<String>) {
    let mut identities: HashSet<_> = entries.iter().map(|entry| entry.id.clone()).collect();
    let mut seen = HashMap::<String, usize>::new();
    for path in &source.paths {
        let mut occurrences = HashMap::<String, usize>::new();
        let scan = records::visit_records(path, 0, |_, offset, value| {
            let Some((mut id, message, projected)) = supplement(&value) else { return Ok(ControlFlow::Continue(())); };
            if projected {
                if !identities.insert(id.clone()) { return Ok(ControlFlow::Continue(())); }
            } else {
                let occurrence = occurrences.entry(id.clone()).or_default();
                *occurrence += 1;
                if seen.get(&id).is_some_and(|count| *count >= *occurrence) { return Ok(ControlFlow::Continue(())); }
                seen.insert(id.clone(), *occurrence);
                id = format!("{id}-{occurrence}");
            }
            entries.push(Entry { id, at: message.at, location: Location::Jsonl {
                path: path.clone(), offset, digest: parser::record_identity(&value) } });
            Ok(ControlFlow::Continue(()))
        });
        match scan { Ok(warnings) => issues.extend(warnings), Err(error) => issues.push(error) }
    }
}

fn native_index(connection: &Connection, thread: &str) -> Result<(Vec<Entry>, Vec<String>), String> {
    let mut statement = connection.prepare(
        "SELECT turn_id,item_id,created_at_ms,CASE WHEN length(CAST(item_json AS BLOB))<=?2 THEN item_json END,0,rollout_ordinal FROM thread_items WHERE thread_id=?1 \
         UNION ALL SELECT '',r.item_id,r.created_at_ms,CASE WHEN length(CAST(r.item_json AS BLOB))<=?2 THEN r.item_json END,1,r.rollout_ordinal FROM thread_realtime_items r \
         WHERE r.thread_id=?1 AND NOT EXISTS (SELECT 1 FROM thread_items i WHERE i.thread_id=r.thread_id AND i.item_id=r.item_id) \
         ORDER BY 6,3,2").map_err(|error| error.to_string())?;
    let mut rows = statement.query(params![thread, records::MAX_RECORD_BYTES]).map_err(|error| error.to_string())?;
    let mut entries = Vec::new();
    let mut issues = Vec::new();
    while let Some(row) = rows.next().map_err(|error| error.to_string())? {
        match native_entry(row) {
            Ok(Some(entry)) => entries.push(entry),
            Ok(None) => {},
            Err(error) => issues.push(format!("会话 {thread} 的一条原生消息不可读: {error}")),
        }
    }
    Ok((entries, issues))
}

fn native_entry(row: &rusqlite::Row<'_>) -> Result<Option<Entry>, String> {
    let raw: String = row.get::<_, Option<String>>(3).map_err(|error| error.to_string())?
        .ok_or("原生消息超过 32 MiB 单条读取上限")?;
    let value: Value = serde_json::from_str(&raw).map_err(|error| error.to_string())?;
    if parser::message_from(&value).is_none() { return Ok(None); }
    let milliseconds: i64 = row.get(2).map_err(|error| error.to_string())?;
    Ok(Some(Entry { id: parser::native_identity(&value),
        at: Utc.timestamp_millis_opt(milliseconds).single().map(|at| at.to_rfc3339()),
        location: Location::Native { turn: row.get(0).map_err(|error| error.to_string())?,
            item: row.get(1).map_err(|error| error.to_string())?, realtime: row.get(4).map_err(|error| error.to_string())? } }))
}

fn supplement(value: &Value) -> Option<(String, SessionMessage, bool)> {
    if value.get("type").and_then(Value::as_str) == Some("event_msg") {
        let payload = value.get("payload")?;
        if payload.get("type").and_then(Value::as_str) != Some("item_completed") { return None; }
        let item = payload.get("item")?;
        return Some((parser::native_identity(item), parser::message_from(value)?, true));
    }
    // Raw protocol fields can retain instructions, encrypted reasoning or messages
    // absent from the native UI projection; only identical projected items collapse.
    Some((format!("protocol-{}", parser::record_identity(value)), parser::message_from(value)?, false))
}

fn load(connection: &Connection, thread: &str, entry: &Entry) -> Result<SessionMessage, String> {
    let value = match &entry.location {
        Location::Native { turn, item, realtime } => {
            let sql = if *realtime { "SELECT item_json FROM thread_realtime_items WHERE thread_id=?1 AND item_id=?3" }
                else { "SELECT item_json FROM thread_items WHERE thread_id=?1 AND turn_id=?2 AND item_id=?3" };
            let raw: String = connection.query_row(sql, params![thread,turn,item], |row| row.get(0)).map_err(|error| error.to_string())?;
            serde_json::from_str::<Value>(&raw).map_err(|error| error.to_string())?
        }
        Location::Jsonl { path, offset, digest } => {
            let mut found = None;
            records::visit_records(path, *offset, |_, _, value| { found = Some(value); Ok(ControlFlow::Break(())) })?;
            let value = found.ok_or("原会话记录已经改变，请重新打开会话")?;
            if parser::record_identity(&value) != *digest { return Err("原会话记录已经改变，请重新打开会话".into()); }
            value
        }
    };
    parser::message_from(&value).ok_or_else(|| "原消息内容已不可读".into())
}

#[cfg(test)]
mod tests;
