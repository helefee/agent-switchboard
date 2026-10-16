//! CLI event identity and exact-session usage. Neither stderr nor unrelated
//! sessions can provide an answer, model or token count for a probe.
use crate::codex_metering::{merge_token_event, visit_token_usage, SessionTokenSummary};
use serde_json::Value;
use std::path::Path;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

pub(super) struct ProbeEvents {
    pub session_id: String,
    pub completed: bool,
    pub failure: Option<String>,
}

pub(super) fn parse_events(output: &str) -> Result<ProbeEvents, String> {
    let mut session_id = None;
    let mut completed = false;
    let mut failure = None;
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let event: Value = serde_json::from_str(line)
            .map_err(|_| "Codex CLI 未返回有效的 JSON 事件".to_string())?;
        match event.get("type").and_then(Value::as_str) {
            Some("thread.started") => {
                let id = event.get("thread_id").and_then(Value::as_str)
                    .and_then(|id| uuid::Uuid::parse_str(id).ok())
                    .ok_or_else(|| "Codex CLI 返回了无效的会话 ID".to_string())?
                    .hyphenated().to_string();
                if session_id.as_ref().is_some_and(|previous| previous != &id) {
                    return Err("Codex CLI 返回了多个会话 ID".to_string());
                }
                session_id = Some(id);
            }
            Some("turn.completed") => completed = true,
            Some("turn.failed") => {
                failure = Some(diagnostic(event.pointer("/error/message")
                    .and_then(Value::as_str).unwrap_or("Codex 执行失败")));
            }
            _ => {}
        }
    }
    Ok(ProbeEvents {
        session_id: session_id.ok_or_else(|| "Codex CLI 未返回会话 ID".to_string())?,
        completed,
        failure,
    })
}

pub(super) fn diagnostic(message: &str) -> String {
    let scrubbed = asb_core::adapter::scrub_message(message.to_string());
    let tail: String = scrubbed.chars().rev().take(1000).collect();
    tail.chars().rev().collect::<String>().trim().to_string()
}

pub(super) fn read_usage(session_id: &str) -> Result<SessionTokenSummary, String> {
    let root = crate::session_manager::codex_root()?;
    let mut attempts = 0;
    loop {
        let result = read_session_usage(&root, session_id);
        attempts += 1;
        if result.is_ok() || attempts == 3 { return result; }
        std::thread::sleep(Duration::from_millis(400));
    }
}

fn read_session_usage(root: &Path, session_id: &str) -> Result<SessionTokenSummary, String> {
    let mut events = HashMap::new();
    let mut issues = Vec::new();
    for session_root in crate::local_state::codex_paths::session_roots(root) {
        let files = crate::session_manager::collect_session_files(&session_root);
        issues.extend(files.issues);
        for path in files.paths {
            let Some(identity) = crate::session_manager::parser::session_identity(&path).map_err(|error| error.to_string())? else { continue; };
            if identity.id != session_id { continue; }
            issues.extend(visit_token_usage(&path, session_id, |event| {
                if event.owner == session_id { merge_token_event(&mut events, event)?; }
                Ok(())
            })?);
        }
    }
    if !issues.is_empty() { return Err(issues.join("；")); }
    if events.is_empty() { return Err("未找到本次检测会话的真实调用用量记录".into()); }
    let mut summary = SessionTokenSummary { input: 0, cached_input: 0, output: 0, reasoning_output: Some(0), total: None, model: None };
    let mut models = HashSet::new();
    for event in events.into_values() {
        summary.input = summary.input.checked_add(event.tokens.input).ok_or("输入计数溢出")?;
        summary.cached_input = summary.cached_input.checked_add(event.tokens.cached_input).ok_or("缓存计数溢出")?;
        summary.output = summary.output.checked_add(event.tokens.output).ok_or("输出计数溢出")?;
        summary.reasoning_output = Some(summary.reasoning_output.unwrap_or(0).checked_add(event.tokens.reasoning_output).ok_or("推理计数溢出")?);
        models.insert(event.model);
    }
    if models.len() == 1 { summary.model = models.into_iter().next().flatten(); }
    summary.total = Some(summary.input.checked_add(summary.output).ok_or("总计数溢出")?);
    Ok(summary)
}

#[cfg(test)]
mod tests;
