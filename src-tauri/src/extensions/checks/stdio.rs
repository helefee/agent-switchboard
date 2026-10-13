//! stdio transport with bounded reads/writes and process-tree cleanup.
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc::{self, Receiver, SyncSender};
use std::time::Duration;
use serde_json::Value;
use super::protocol::{self, Connection};
use super::{CheckDeadline, CheckError, ProbeOutcome, MAX_OUTPUT_BYTES, RECEIVE_POLL};
use crate::process_control::terminate_process_tree;

type WriteReply = SyncSender<Result<(), CheckError>>;
struct WriteRequest { bytes: Vec<u8>, reply: WriteReply }
enum ReaderMessage { Line(Vec<u8>), Closed, Error(CheckError) }
struct StdioConnection {
    writer: SyncSender<WriteRequest>,
    reader: Receiver<ReaderMessage>,
    bytes_read: usize,
}

impl StdioConnection {
    fn send(&self, value: &Value, deadline: &CheckDeadline) -> Result<(), CheckError> {
        deadline.remaining()?;
        let mut bytes = serde_json::to_vec(value).map_err(|_| CheckError::protocol("请求无法编码"))?;
        bytes.push(b'\n');
        let (reply, receiver) = mpsc::sync_channel(1);
        self.writer.try_send(WriteRequest { bytes, reply })
            .map_err(|_| CheckError::transport("无法写入检测命令"))?;
        loop {
            match receiver.recv_timeout(deadline.remaining()?.min(RECEIVE_POLL)) {
                Ok(result) => return result,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
                Err(mpsc::RecvTimeoutError::Disconnected) => return Err(CheckError::closed()),
            }
        }
    }
}

impl Connection for StdioConnection {
    fn request(&mut self, request: &Value, deadline: &CheckDeadline) -> Result<Value, CheckError> {
        self.send(request, deadline)?;
        loop {
            match self.reader.recv_timeout(deadline.remaining()?.min(RECEIVE_POLL)) {
                Ok(ReaderMessage::Line(line)) => {
                    self.bytes_read = self.bytes_read.saturating_add(line.len() + 1);
                    if self.bytes_read > MAX_OUTPUT_BYTES { return Err(CheckError::output_limit()); }
                    let value: Value = serde_json::from_slice(&line)
                        .map_err(|_| CheckError::protocol("服务标准输出不是有效的 JSON-RPC 消息"))?;
                    if protocol::is_response(&value, &request["id"]) { return Ok(value); }
                    if let Some(reply) = protocol::server_request_reply(&value) {
                        self.send(&reply, deadline)?;
                    }
                }
                Ok(ReaderMessage::Closed) | Err(mpsc::RecvTimeoutError::Disconnected) =>
                    return Err(CheckError::closed()),
                Ok(ReaderMessage::Error(error)) => return Err(error),
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    }

    fn notify(&mut self, notification: &Value, deadline: &CheckDeadline) -> Result<(), CheckError> {
        self.send(notification, deadline)
    }

    fn set_protocol_version(&mut self, _version: &str) {}
}

pub(super) fn check_stdio(
    command: &str, args: &[String], env: &BTreeMap<String, String>,
    working_dir: &Path, deadline: &CheckDeadline, initialize_timeout: Duration,
) -> ProbeOutcome {
    if let Err(error) = deadline.remaining() { return ProbeOutcome::for_error(None, error); }
    let program = match program_path(command, env, working_dir) {
        Ok(program) => program,
        Err(error) => return ProbeOutcome::failed("spawn", error.message),
    };
    let mut builder = Command::new(program);
    builder.args(args).envs(env).current_dir(working_dir)
        .stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::null());
    crate::process_control::suppress_window(&mut builder);
    let mut child = match builder.spawn() {
        Ok(child) => child,
        Err(error) => return ProbeOutcome::failed("spawn",
            format!("无法启动检测命令（{:?}）", error.kind())),
    };
    drive_stdio_check(&mut child, deadline, initialize_timeout)
}

fn drive_stdio_check(child: &mut Child, deadline: &CheckDeadline, initialize_timeout: Duration) -> ProbeOutcome {
    let (Some(mut stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
        terminate_process_tree(child);
        return ProbeOutcome::failed("spawn", "检测命令未提供标准输入和输出");
    };
    let (write_tx, write_rx) = mpsc::sync_channel::<WriteRequest>(1);
    let writer = std::thread::Builder::new().name("mcp-stdin".into()).spawn(move || {
        while let Ok(request) = write_rx.recv() {
            let result = stdin.write_all(&request.bytes).and_then(|()| stdin.flush())
                .map_err(|_| CheckError::transport("无法写入检测命令"));
            let failed = result.is_err();
            let _ = request.reply.send(result);
            if failed { break; }
        }
    });
    let writer = match writer {
        Ok(writer) => writer,
        Err(_) => {
            terminate_process_tree(child);
            return ProbeOutcome::failed("spawn", "无法启动检测输入线程");
        }
    };
    let (read_tx, read_rx) = mpsc::sync_channel(4);
    let reader = std::thread::Builder::new().name("mcp-stdout".into())
        .spawn(move || read_stdio_lines(stdout, read_tx));
    let reader = match reader {
        Ok(reader) => reader,
        Err(_) => {
            drop(write_tx);
            terminate_process_tree(child);
            let _ = writer.join();
            return ProbeOutcome::failed("spawn", "无法启动检测输出线程");
        }
    };
    let mut connection = StdioConnection { writer: write_tx, reader: read_rx, bytes_read: 0 };
    let probe = protocol::probe(&mut connection, deadline, initialize_timeout);
    drop(connection);
    terminate_process_tree(child);
    let _ = writer.join();
    let _ = reader.join();
    probe
}

fn read_stdio_lines(stdout: std::process::ChildStdout, sender: SyncSender<ReaderMessage>) {
    let mut reader = BufReader::new(stdout);
    loop {
        let message = match read_line_capped(&mut reader, MAX_OUTPUT_BYTES) {
            Ok(Some(bytes)) => ReaderMessage::Line(bytes),
            Ok(None) => ReaderMessage::Closed,
            Err(error) => ReaderMessage::Error(if error.kind() == std::io::ErrorKind::InvalidData {
                CheckError::output_limit()
            } else { CheckError::transport("服务输出无法读取") }),
        };
        let done = !matches!(message, ReaderMessage::Line(_));
        if sender.send(message).is_err() || done { break; }
    }
}

fn program_path(command: &str, env: &BTreeMap<String, String>, working_dir: &Path) -> Result<PathBuf, CheckError> {
    let path = Path::new(command);
    if path.is_absolute() { return Ok(path.to_path_buf()); }
    if command.contains('/') || command.contains('\\') { return Ok(working_dir.join(path)); }
    #[cfg(windows)]
    {
        if command.eq_ignore_ascii_case("cmd") || command.eq_ignore_ascii_case("cmd.exe") {
            let root = std::env::var_os("SystemRoot")
                .ok_or_else(|| CheckError::transport("无法定位 Windows 命令解释器"))?;
            return Ok(PathBuf::from(root).join("System32").join("cmd.exe"));
        }
        let path_value = env.iter().find(|(key, _)| key.eq_ignore_ascii_case("PATH"))
            .map(|(_, value)| std::ffi::OsString::from(value.as_str()))
            .or_else(|| std::env::var_os("PATH")).unwrap_or_default();
        let extensions = env.iter().find(|(key, _)| key.eq_ignore_ascii_case("PATHEXT"))
            .map(|(_, value)| value.clone()).or_else(|| std::env::var("PATHEXT").ok())
            .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".into());
        for directory in std::env::split_paths(&path_value) {
            let directory = working_dir.join(directory);
            for candidate in std::iter::once(directory.join(command)).chain(
                extensions.split(';').map(|extension| directory.join(format!("{command}{extension}"))),
            ) {
                if candidate.is_file() { return Ok(candidate); }
            }
        }
        return Err(CheckError::transport("检测命令未在目标环境的 PATH 中找到"));
    }
    #[cfg(not(windows))]
    { let _ = env; Ok(path.to_path_buf()) }
}

#[cfg(test)]
mod tests;

pub(super) fn read_line_capped<R: BufRead>(
    reader: &mut R,
    max_bytes: usize,
) -> std::io::Result<Option<Vec<u8>>> {
    let mut line = Vec::new();
    loop {
        let buffer = reader.fill_buf()?;
        if buffer.is_empty() {
            return if line.is_empty() {
                Ok(None)
            } else {
                Ok(Some(line))
            };
        }
        if let Some(newline) = buffer.iter().position(|byte| *byte == b'\n') {
            if line.len().saturating_add(newline) > max_bytes {
                reader.consume(newline + 1);
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "line exceeds cap",
                ));
            }
            line.extend_from_slice(&buffer[..newline]);
            reader.consume(newline + 1);
            if line.last() == Some(&b'\r') {
                line.pop();
            }
            return Ok(Some(line));
        }
        let length = buffer.len();
        if line.len().saturating_add(length) > max_bytes {
            reader.consume(length);
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "line exceeds cap",
            ));
        }
        line.extend_from_slice(buffer);
        reader.consume(length);
    }
}
