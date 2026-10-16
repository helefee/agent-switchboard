use super::{parser, records, SessionMessage};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Seek, SeekFrom};
use std::ops::ControlFlow;
use std::path::PathBuf;

struct Entry { file: usize, offset: u64, id: String, digest: String, order_at: Option<String> }

pub(super) fn visit(
    paths: &[PathBuf],
    visitor: &mut impl FnMut(SessionMessage) -> Result<ControlFlow<()>, String>,
) -> Result<Vec<String>, String> {
    let _guard = parser::SCAN_GATE.lock().map_err(|_| "会话读取锁不可用")?;
    if paths.len() <= 1 { return scan(paths, &mut |_, message| visitor(message)); }
    let mut entries = Vec::new();
    let mut issues = scan(paths, &mut |entry, _| { entries.push(entry); Ok(ControlFlow::Continue(())) })?;
    entries.sort_by(|left, right| left.order_at.cmp(&right.order_at));
    let mut current = None;
    for entry in entries {
        match load(paths, &entry, &mut current) {
            Ok(message) => { if visitor(message)?.is_break() { break; } }
            Err(error) => issues.push(error),
        }
    }
    Ok(issues)
}

fn scan(
    paths: &[PathBuf],
    visitor: &mut impl FnMut(Entry, SessionMessage) -> Result<ControlFlow<()>, String>,
) -> Result<Vec<String>, String> {
    let mut seen = HashMap::<String, usize>::new();
    let mut issues = Vec::new();
    let mut stopped = false;
    for (file, path) in paths.iter().enumerate() {
        let mut occurrences = HashMap::<String, usize>::new();
        let mut prefix = Sha256::new();
        let mut order_at = None;
        let mut visitor_failure = None;
        let scan = records::visit_records(path, 0, |_, offset, value| {
            prefix.update(value.to_string().as_bytes());
            prefix.update(b"\n");
            let Some(mut message) = parser::message_from(&value) else { return Ok(ControlFlow::Continue(())); };
            let digest = parser::record_identity(&value);
            let occurrence = occurrences.entry(digest.clone()).or_default();
            *occurrence += 1;
            if message.at.is_some() { order_at = message.at.clone(); }
            if seen.get(&digest).is_some_and(|count| *count >= *occurrence) { return Ok(ControlFlow::Continue(())); }
            message.id = format!("raw-{:x}-{}", prefix.clone().finalize(), *occurrence);
            seen.insert(digest.clone(), *occurrence);
            let entry = Entry { file, offset, id: message.id.clone(), digest, order_at: order_at.clone() };
            let flow = visitor(entry, message).map_err(|error| { visitor_failure = Some(error.clone()); error })?;
            stopped = flow.is_break();
            Ok(flow)
        });
        if let Some(error) = visitor_failure { return Err(error); }
        match scan { Ok(warnings) => issues.extend(warnings), Err(error) => issues.push(error) }
        if stopped { break; }
    }
    Ok(issues)
}

fn load(paths: &[PathBuf], entry: &Entry, current: &mut Option<(usize, BufReader<File>)>) -> Result<SessionMessage, String> {
    let path = &paths[entry.file];
    if current.as_ref().is_none_or(|(file, _)| *file != entry.file) {
        let file = File::open(path).map_err(|error| format!("无法读取会话记录 {}: {error}", path.display()))?;
        *current = Some((entry.file, BufReader::new(file)));
    }
    let (_, reader) = current.as_mut().ok_or("会话读取位置不可用")?;
    reader.seek(SeekFrom::Start(entry.offset)).map_err(|error| error.to_string())?;
    let mut record = None;
    records::visit_reader(&path.display().to_string(), entry.offset, reader, |_, _, value| {
        record = Some(value); Ok(ControlFlow::Break(()))
    })?;
    let record = record.ok_or("原会话记录已经改变，请重新打开会话")?;
    if parser::record_identity(&record) != entry.digest { return Err("原会话记录已经改变，请重新打开会话".into()); }
    let mut message = parser::message_from(&record).ok_or("原会话记录已经不可读")?;
    message.id = entry.id.clone();
    Ok(message)
}
