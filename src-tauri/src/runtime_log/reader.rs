use super::{is_runtime_log_file, RuntimeLogEntry, RuntimeLogRead, RuntimeLogSeverity, MAX_VISIBLE_ENTRIES};
use std::cmp::Ordering;
use std::collections::BinaryHeap;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read};
use std::path::Path;

const MAX_LINE_BYTES: u64 = 16 * 1024;

struct Candidate { entry: RuntimeLogEntry, sequence: u64 }

impl PartialEq for Candidate {
    fn eq(&self, other: &Self) -> bool { self.entry.at == other.entry.at && self.sequence == other.sequence }
}
impl Eq for Candidate {}
impl PartialOrd for Candidate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> { Some(self.cmp(other)) }
}
impl Ord for Candidate {
    fn cmp(&self, other: &Self) -> Ordering {
        other.entry.at.cmp(&self.entry.at).then_with(|| self.sequence.cmp(&other.sequence))
    }
}

pub(super) fn list_from_dir(directory: &Path, level: Option<RuntimeLogSeverity>) -> Result<RuntimeLogRead, String> {
    let files = match fs::read_dir(directory) {
        Ok(files) => files,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(RuntimeLogRead { entries: Vec::new(), total: 0, skipped: 0 }),
        Err(_) => return Err("应用日志目录不可读".into()),
    };
    let mut latest = BinaryHeap::<Candidate>::with_capacity(MAX_VISIBLE_ENTRIES);
    let mut sequence = 0;
    let mut total = 0;
    let mut skipped = 0;
    for file in files {
        let file = file.map_err(|_| "应用日志目录不可读")?;
        if !file.file_type().map_err(|_| "应用日志目录不可读")?.is_file()
            || !is_runtime_log_file(&file.path()) { continue; }
        let mut reader = BufReader::new(File::open(file.path()).map_err(|_| "应用日志不可读")?);
        let mut line = Vec::new();
        while let Some(valid_length) = read_line(&mut reader, &mut line).map_err(|_| "应用日志不可读")? {
            if !valid_length { skipped += 1; continue; }
            let Ok(entry) = serde_json::from_slice::<RuntimeLogEntry>(&line) else { skipped += 1; continue; };
            if !entry.is_valid() { skipped += 1; continue; }
            if level.is_some_and(|level| entry.level != level) { continue; }
            total += 1;
            sequence += 1;
            let candidate = Candidate { entry, sequence };
            if latest.len() < MAX_VISIBLE_ENTRIES { latest.push(candidate); }
            else if latest.peek().is_some_and(|oldest| candidate < *oldest) {
                latest.pop(); latest.push(candidate);
            }
        }
    }
    let mut candidates = latest.into_vec();
    candidates.sort_by(|left, right| right.entry.at.cmp(&left.entry.at).then(left.sequence.cmp(&right.sequence)));
    Ok(RuntimeLogRead { entries: candidates.into_iter().map(|candidate| candidate.entry).collect(), total, skipped })
}

fn read_line(reader: &mut impl BufRead, line: &mut Vec<u8>) -> std::io::Result<Option<bool>> {
    line.clear();
    let count = (&mut *reader).take(MAX_LINE_BYTES + 1).read_until(b'\n', line)?;
    if count == 0 { return Ok(None); }
    if count as u64 <= MAX_LINE_BYTES { return Ok(Some(true)); }
    if line.last() != Some(&b'\n') {
        loop {
            let bytes = reader.fill_buf()?;
            if bytes.is_empty() { break; }
            let end = bytes.iter().position(|byte| *byte == b'\n');
            let consumed = end.map_or(bytes.len(), |index| index + 1);
            reader.consume(consumed);
            if end.is_some() { break; }
        }
    }
    Ok(Some(false))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oversized_lines_are_discarded_without_consuming_the_following_record() {
        let mut input = vec![b'x'; MAX_LINE_BYTES as usize * 3];
        input.extend_from_slice(b"\n{\"valid\":true}\n");
        let mut reader = std::io::Cursor::new(input);
        let mut line = Vec::new();
        assert_eq!(read_line(&mut reader, &mut line).unwrap(), Some(false));
        assert!(line.len() <= MAX_LINE_BYTES as usize + 1);
        assert_eq!(read_line(&mut reader, &mut line).unwrap(), Some(true));
        assert_eq!(line, b"{\"valid\":true}\n");
        assert_eq!(read_line(&mut reader, &mut line).unwrap(), None);
    }
}
