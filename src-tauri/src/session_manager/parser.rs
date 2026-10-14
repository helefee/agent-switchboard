use super::{SessionMessage, SessionMeta, TITLE_LIMIT};
use asb_core::contracts::AppKind;
use chrono::{DateTime, Utc};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, Read};
use std::ops::ControlFlow;
use std::path::Path;
use std::sync::Mutex;

const SUMMARY_LIMIT: usize = 180;
/// Every metadata field below is first-match-wins, so once all five are
/// collected the remaining lines cannot change the result. This cap bounds
/// the per-file scan cost when some fields never appear; records past the cap
/// are ignored and the title falls back as if they were absent.
pub(super) const METADATA_LINE_LIMIT: usize = 32;
pub(super) const MAX_RECORD_BYTES: u64 = 32 * 1024 * 1024;
static SCAN_GATE: Mutex<()> = Mutex::new(());

fn read_record(reader: &mut impl BufRead, line: &mut String) -> io::Result<bool> {
    line.clear();
    let bytes = (&mut *reader).take(MAX_RECORD_BYTES + 1).read_line(line)?;
    if bytes as u64 > MAX_RECORD_BYTES {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "会话单条记录超过 32 MiB 读取上限"));
    }
    Ok(bytes != 0)
}

pub(super) fn parse_session(app: AppKind, path: &Path) -> Result<SessionMeta, io::Error> {
    let _guard = SCAN_GATE.lock().map_err(|_| io::Error::other("会话读取锁不可用"))?;
    let mut session_id = None;
    let mut project_dir = None;
    let mut created_at = None;
    let mut custom_title = None;
    let mut first_user_message = None;

    let mut reader = BufReader::new(File::open(path)?);
    let mut line = String::new();
    for index in 0..METADATA_LINE_LIMIT {
        if !read_record(&mut reader, &mut line)? { break; }
        if line.trim().is_empty() { continue; }
        let value = serde_json::from_str::<Value>(&line).map_err(|error| {
            io::Error::new(io::ErrorKind::InvalidData, format!("第 {} 行 JSON 无效: {error}", index + 1))
        })?;
        session_id = session_id.or_else(|| session_id_from(&value));
        project_dir = project_dir.or_else(|| project_dir_from(&value));
        created_at = created_at.or_else(|| timestamp_from(&value));
        custom_title = custom_title.or_else(|| custom_title_from(&value));
        if first_user_message.is_none() {
            first_user_message = message_from(&value)
                .filter(|message| message.role == "user")
                .filter(|message| useful_user_content(&message.content))
                .map(|message| clamp_text(&message.content, SUMMARY_LIMIT));
        }
        if session_id.is_some()
            && project_dir.is_some()
            && created_at.is_some()
            && custom_title.is_some()
            && first_user_message.is_some()
        {
            break;
        }
    }

    let session_id = session_id
        .filter(|id| valid_session_id(id))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing session id"))?;
    let title = custom_title
        .or_else(|| first_user_message.clone())
        .or_else(|| project_dir.as_deref().and_then(project_name))
        .unwrap_or_else(|| session_id.clone());
    let last_active_at = fs::metadata(path)
        .ok()
        .and_then(|metadata| metadata.modified().ok())
        .map(system_time_iso);

    Ok(SessionMeta {
        app,
        resume_command: super::resume::resume_command(app, &session_id),
        session_id,
        title: clamp_text(&title, TITLE_LIMIT),
        summary: clamp_text(
            first_user_message.as_deref().unwrap_or(&title),
            SUMMARY_LIMIT,
        ),
        project_dir,
        created_at,
        last_active_at,
        alias: None,
        pinned: false,
        tags: Vec::new(),
    })
}

pub(crate) fn session_id_in_session_file(path: &Path) -> io::Result<Option<String>> {
    let _guard = SCAN_GATE.lock().map_err(|_| io::Error::other("会话读取锁不可用"))?;
    let mut reader = BufReader::new(File::open(path)?);
    let mut line = String::new();
    for _ in 0..METADATA_LINE_LIMIT {
        if !read_record(&mut reader, &mut line)? { break; }
        let Ok(value) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        if let Some(session_id) = session_id_from(&value).filter(|id| valid_session_id(id)) {
            return Ok(Some(session_id));
        }
    }
    Ok(None)
}

pub(super) fn visit_messages(
    path: &Path,
    mut visitor: impl FnMut(SessionMessage) -> Result<ControlFlow<()>, String>,
) -> Result<(), String> {
    let _guard = SCAN_GATE.lock().map_err(|_| "会话读取锁不可用")?;
    let file = File::open(path)
        .map_err(|error| format!("无法读取会话记录 {}: {error}", path.display()))?;
    let mut reader = BufReader::new(file);
    let mut prefix = Sha256::new();
    let mut line = String::new();
    let mut line_number = 0;
    loop {
        let read = read_record(&mut reader, &mut line)
            .map_err(|error| format!("无法读取 {} 第 {} 行: {error}", path.display(), line_number + 1))?;
        if !read { break; }
        line_number += 1;
        // Hash the complete prefix: appending preserves IDs; edits cannot
        // redirect an old search hit to a different occurrence of a message.
        prefix.update(line.trim_end_matches(['\r', '\n']).as_bytes());
        prefix.update(b"\n");
        if line.trim().is_empty() { continue; }
        let value = serde_json::from_str::<Value>(&line)
            .map_err(|error| format!("会话 {} 第 {line_number} 行 JSON 无效: {error}", path.display()))?;
        if let Some(mut message) = message_from(&value) {
            message.id = format!("{:x}", prefix.clone().finalize());
            if visitor(message)?.is_break() { break; }
        }
    }
    Ok(())
}

pub(super) fn find_message(path: &Path, id: &str) -> Result<SessionMessage, String> {
    let mut found = None;
    visit_messages(path, |message| {
        if message.id == id {
            found = Some(message);
            Ok(ControlFlow::Break(()))
        } else {
            Ok(ControlFlow::Continue(()))
        }
    })?;
    found.ok_or_else(|| "原消息已改变或不存在；请重新打开会话".into())
}

#[cfg(test)]
pub(super) fn collect_messages(path: &Path) -> Result<Vec<SessionMessage>, String> {
    let mut messages = Vec::new();
    visit_messages(path, |message| {
        messages.push(message);
        Ok(ControlFlow::Continue(()))
    })?;
    Ok(messages)
}

fn message_from(value: &Value) -> Option<SessionMessage> {
    let envelope = if value.get("type").and_then(Value::as_str) == Some("response_item") {
        let payload = value.get("payload").unwrap_or(value);
        if matches!(payload.get("type").and_then(Value::as_str),
            Some("function_call_output" | "custom_tool_call_output")) {
            let content = payload.get("output").and_then(content_text)?;
            if content.trim().is_empty() { return None; }
            return Some(SessionMessage {
                id: String::new(), role: "tool".into(), content, at: timestamp_from(value),
            });
        }
        payload
    } else {
        value
    };
    let message = envelope.get("message").unwrap_or(envelope);
    let role = message
        .get("role")
        .or_else(|| envelope.get("role"))
        .and_then(Value::as_str)
        .or_else(|| match envelope.get("type").and_then(Value::as_str) {
            Some("user") => Some("user"),
            Some("assistant") => Some("assistant"),
            Some("system") => Some("system"),
            _ => None,
        })?;
    let content = message
        .get("content")
        .or_else(|| envelope.get("content"))
        .and_then(content_text)?;
    if content.trim().is_empty() {
        return None;
    }
    Some(SessionMessage {
        id: String::new(),
        role: role.to_string(),
        content,
        at: timestamp_from(value),
    })
}

fn content_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(items) => {
            let text = items
                .iter()
                .filter_map(content_text)
                .collect::<Vec<_>>()
                .join("\n");
            (!text.is_empty()).then_some(text)
        }
        Value::Object(object) => object
            .get("text")
            .or_else(|| object.get("content"))
            .or_else(|| object.get("value"))
            .and_then(content_text),
        _ => None,
    }
}

fn session_id_from(value: &Value) -> Option<String> {
    let payload = value.get("payload").unwrap_or(value);
    value
        .get("sessionId")
        .or_else(|| value.get("session_id"))
        .or_else(|| payload.get("id"))
        .or_else(|| payload.get("sessionId"))
        .or_else(|| payload.get("session_id"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn project_dir_from(value: &Value) -> Option<String> {
    let payload = value.get("payload").unwrap_or(value);
    value
        .get("cwd")
        .or_else(|| payload.get("cwd"))
        .and_then(Value::as_str)
        .filter(|path| !path.is_empty())
        .map(str::to_string)
}

fn timestamp_from(value: &Value) -> Option<String> {
    let payload = value.get("payload").unwrap_or(value);
    value
        .get("timestamp")
        .or_else(|| payload.get("timestamp"))
        .and_then(Value::as_str)
        .filter(|timestamp| !timestamp.is_empty())
        .map(str::to_string)
}

fn custom_title_from(value: &Value) -> Option<String> {
    value
        .get("customTitle")
        .or_else(|| value.get("custom_title"))
        .and_then(Value::as_str)
        .filter(|title| !title.trim().is_empty())
        .map(str::to_string)
}

pub(super) fn valid_session_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

/// Applies the list-title limit to a title sourced outside the transcript.
pub(super) fn clamp_title(text: &str) -> String {
    clamp_text(text, TITLE_LIMIT)
}

fn clamp_text(text: &str, limit: usize) -> String {
    let mut characters = super::text::compact_chars(text);
    let mut result = characters.by_ref().take(limit).collect::<String>();
    if characters.next().is_some() { result.push('…'); }
    result
}

fn useful_user_content(content: &str) -> bool {
    !content.starts_with("<local-command-caveat>") && !content.starts_with("<command-name>")
}

fn project_name(path: &str) -> Option<String> {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(str::to_string)
}

fn system_time_iso(time: std::time::SystemTime) -> String {
    DateTime::<Utc>::from(time).to_rfc3339()
}

#[cfg(test)]
mod read_tests {
    use super::*;

    #[test]
    fn refuses_oversized_records_before_allocating_an_entire_input() {
        let input = std::io::repeat(b'x').take(MAX_RECORD_BYTES + 100);
        let mut reader = BufReader::new(input);
        let mut line = String::new();
        assert_eq!(read_record(&mut reader, &mut line).unwrap_err().kind(), io::ErrorKind::InvalidData);
        assert_eq!(line.len(), MAX_RECORD_BYTES as usize + 1);
    }
}
