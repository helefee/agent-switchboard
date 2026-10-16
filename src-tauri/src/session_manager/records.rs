use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::ops::ControlFlow;
use std::path::Path;

pub(super) const MAX_RECORD_BYTES: u64 = 32 * 1024 * 1024;

pub(crate) fn visit_records(
    path: &Path,
    offset: u64,
    visitor: impl FnMut(usize, u64, Value) -> Result<ControlFlow<()>, String>,
) -> Result<Vec<String>, String> {
    let mut file = File::open(path)
        .map_err(|error| format!("无法读取会话记录 {}: {error}", path.display()))?;
    file.seek(SeekFrom::Start(offset)).map_err(|error| error.to_string())?;
    visit_reader(&path.display().to_string(), offset, BufReader::new(file), visitor)
}

pub(crate) fn visit_reader(
    label: &str,
    offset: u64,
    mut reader: impl BufRead,
    mut visitor: impl FnMut(usize, u64, Value) -> Result<ControlFlow<()>, String>,
) -> Result<Vec<String>, String> {
    let mut line = Vec::new();
    let mut issues = Vec::new();
    let mut ordinal = 0;
    let mut position = offset;
    loop {
        line.clear();
        let count = (&mut reader).take(MAX_RECORD_BYTES + 1).read_until(b'\n', &mut line)
            .map_err(|error| format!("读取 {} 第 {} 条记录失败: {error}", label, ordinal + 1))?;
        if count == 0 { break; }
        ordinal += 1;
        let record_offset = position;
        position += count as u64;
        if count as u64 > MAX_RECORD_BYTES {
            issues.push(format!("{} 第 {ordinal} 条记录超过 32 MiB 读取上限，其他可读记录已保留", label));
            if line.last() != Some(&b'\n') {
                position += reader.skip_until(b'\n').map_err(|error| format!("跳过超限记录失败: {error}"))? as u64;
            }
            continue;
        }
        if line.iter().all(u8::is_ascii_whitespace) { continue; }
        match serde_json::from_slice::<Value>(&line) {
            Ok(value) => { if visitor(ordinal, record_offset, value)?.is_break() { break; } }
            Err(error) if line.last() != Some(&b'\n') => {
                issues.push(format!("{} 末条记录尚未完整写入，已展示此前完整记录: {error}", label));
                break;
            }
            Err(error) => issues.push(format!("{} 第 {ordinal} 条记录 JSON 无效: {error}", label)),
        }
    }
    Ok(issues)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_oversized_record_does_not_hide_readable_neighbors() {
        let mut bytes = b"{\"valid\":1}\n".to_vec();
        bytes.extend(std::iter::repeat_n(b'x', MAX_RECORD_BYTES as usize + 1));
        bytes.extend_from_slice(b"\n{\"valid\":2}\n");
        let mut values = Vec::new();
        let issues = visit_reader("fixture", 0, std::io::Cursor::new(bytes), |_, _, value| {
            values.push(value["valid"].as_u64().unwrap()); Ok(ControlFlow::Continue(()))
        }).unwrap();
        assert_eq!(values, [1, 2]);
        assert_eq!(issues.len(), 1);
    }
}
