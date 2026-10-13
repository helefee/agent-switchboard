use super::*;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc};
use std::time::{Duration, Instant};
use serde_json::json;

fn stream_server(data: &str) -> (
    String, mpsc::Sender<()>, mpsc::Receiver<()>, std::thread::JoinHandle<std::io::Result<String>>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let url = format!("http://{}/mcp", listener.local_addr().unwrap());
    let (release, released) = mpsc::channel();
    let (ready, streaming) = mpsc::channel();
    let data = data.to_string();
    let thread = std::thread::spawn(move || {
        let started = Instant::now();
        let mut stream = loop {
            match listener.accept() {
                Ok((stream, _)) => break stream,
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock && started.elapsed() < Duration::from_secs(4) =>
                    std::thread::sleep(Duration::from_millis(5)),
                Err(error) => return Err(error),
            }
        };
        stream.set_nonblocking(false)?;
        stream.set_read_timeout(Some(Duration::from_secs(2)))?;
        let mut reader = BufReader::new(stream.try_clone()?);
        let mut headers = String::new();
        let mut length = 0;
        loop {
            let mut line = String::new();
            if reader.read_line(&mut line)? == 0 { return Err(std::io::ErrorKind::UnexpectedEof.into()); }
            if line == "\r\n" { break; }
            let lower = line.to_ascii_lowercase();
            if let Some(value) = lower.strip_prefix("content-length:") { length = value.trim().parse::<usize>().unwrap(); }
            headers.push_str(&lower);
        }
        reader.read_exact(&mut vec![0; length])?;
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\n\r\n{:x}\r\n{}\r\n", data.len(), data)?;
        stream.flush()?;
        let _ = ready.send(());
        // Keep the HTTP body open until the caller proves it has a result.
        let _ = released.recv_timeout(Duration::from_secs(4));
        Ok(headers)
    });
    (url, release, streaming, thread)
}

#[test]
fn response_id_not_eof_ends_a_stream_with_notifications_and_multiline_data() {
    let (url, release, _, server) = stream_server(concat!(
        "data:\n\ndata: {\"jsonrpc\":\"2.0\",\"method\":\"notifications/message\"}\n\n",
        "data: {\"jsonrpc\":\"2.0\",\"id\":\"other\",\"result\":{}}\n\n",
        "data: {\"jsonrpc\":\"2.0\",\n",
        "data: \"id\":\"probe\",\"result\":{\"tools\":[]}}\n\n",
    ));
    let deadline = CheckDeadline::new(Arc::new(AtomicBool::new(false))).limited(Duration::from_secs(2));
    let mut connection = HttpConnection::new(&url, BTreeMap::new(), None).unwrap();
    connection.transport.session = Some("fixture-session".into());
    connection.set_protocol_version("2025-06-18");
    let response = connection.request(&json!({ "jsonrpc": "2.0", "id": "probe", "method": "tools/list" }), &deadline);
    release.send(()).unwrap();
    let headers = server.join().unwrap().unwrap();
    assert_eq!(response.unwrap()["result"]["tools"], json!([]));
    assert!(headers.contains("mcp-session-id: fixture-session"));
    assert!(headers.contains("mcp-protocol-version: 2025-06-18"));
    assert!(!headers.contains("mcp-method:"));
}

#[test]
fn cancellation_interrupts_an_open_http_stream() {
    let (url, release, ready, server) = stream_server(": still running\n\n");
    let flag = Arc::new(AtomicBool::new(false));
    let cancelled = flag.clone();
    let cancel = std::thread::spawn(move || {
        ready.recv_timeout(Duration::from_secs(2)).unwrap();
        cancelled.store(true, Ordering::SeqCst);
    });
    let deadline = CheckDeadline::new(flag).limited(Duration::from_secs(2));
    let mut connection = HttpConnection::new(&url, BTreeMap::new(), None).unwrap();
    let response = connection.request(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }), &deadline);
    release.send(()).unwrap();
    cancel.join().unwrap();
    server.join().unwrap().unwrap();
    assert_eq!(response.unwrap_err().kind, CheckErrorKind::Cancelled);
}
