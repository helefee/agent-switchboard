use serde_json::Value;
use super::{CheckError, MAX_OUTPUT_BYTES};

#[derive(Default)]
pub(super) struct SseDecoder {
    line: Vec<u8>,
    data: Vec<String>,
    skip_lf: bool,
    has_read_line: bool,
    bytes_read: usize,
}

impl SseDecoder {
    pub(super) fn push(&mut self, bytes: &[u8]) -> Result<Vec<Value>, CheckError> {
        self.bytes_read = self.bytes_read.saturating_add(bytes.len());
        if self.bytes_read > MAX_OUTPUT_BYTES { return Err(CheckError::output_limit()); }
        let mut messages = Vec::new();
        for byte in bytes {
            if self.skip_lf {
                self.skip_lf = false;
                if *byte == b'\n' { continue; }
            }
            if *byte == b'\r' || *byte == b'\n' {
                self.skip_lf = *byte == b'\r';
                if let Some(value) = self.dispatch_line()? { messages.push(value); }
            } else {
                self.line.push(*byte);
            }
        }
        Ok(messages)
    }

    fn dispatch_line(&mut self) -> Result<Option<Value>, CheckError> {
        let bytes = std::mem::take(&mut self.line);
        let mut line = std::str::from_utf8(&bytes)
            .map_err(|_| CheckError::protocol("SSE 响应不是 UTF-8"))?;
        if !self.has_read_line {
            self.has_read_line = true;
            line = line.trim_start_matches('\u{feff}');
        }
        if line.is_empty() {
            let data = self.data.join("\n");
            self.data.clear();
            if data.is_empty() { return Ok(None); }
            return serde_json::from_str(&data).map(Some)
                .map_err(|_| CheckError::protocol("SSE 事件不是有效的 JSON-RPC 消息"));
        }
        if let Some(data) = line.strip_prefix("data:") {
            self.data.push(data.strip_prefix(' ').unwrap_or(data).to_string());
        } else if line == "data" {
            self.data.push(String::new());
        }
        Ok(None)
    }
}

#[cfg(test)]
mod tests;
