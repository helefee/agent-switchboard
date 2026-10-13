use super::*;
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

struct Server {
    version: &'static str, capabilities: Value, calls: Vec<Value>, reject_tools: bool,
}

impl Connection for Server {
    fn request(&mut self, request: &Value, deadline: &CheckDeadline) -> Result<Value, CheckError> {
        deadline.remaining()?;
        self.calls.push(request.clone());
        let result = match request["method"].as_str().unwrap() {
            "initialize" => json!({ "protocolVersion": self.version, "capabilities": self.capabilities,
                "serverInfo": { "name": "fixture", "version": "1" } }),
            "tools/list" if self.reject_tools => return Ok(json!({
                "jsonrpc": "2.0", "id": request["id"], "error": { "code": -32601, "message": "unsupported" },
            })),
            "tools/list" if request["params"].get("cursor").is_none() =>
                json!({ "tools": [{ "name": "one" }], "nextCursor": "page-2" }),
            "tools/list" => json!({ "tools": [{ "name": "two" }] }),
            method => panic!("unadvertised method: {method}"),
        };
        Ok(json!({ "jsonrpc": "2.0", "id": request["id"], "result": result }))
    }
    fn notify(&mut self, notification: &Value, _: &CheckDeadline) -> Result<(), CheckError> {
        self.calls.push(notification.clone());
        Ok(())
    }
    fn set_protocol_version(&mut self, version: &str) { assert_eq!(version, self.version); }
}

fn server(version: &'static str, capabilities: Value) -> Server {
    Server { version, capabilities, calls: Vec::new(), reject_tools: false }
}
fn deadline() -> CheckDeadline { CheckDeadline::new(Arc::new(AtomicBool::new(false))) }

#[test]
fn standard_handshake_accepts_negotiated_version_and_lists_only_advertised_capabilities() {
    let mut server = server("2025-06-18", json!({ "tools": {} }));
    let outcome = probe(&mut server, &deadline(), Duration::from_secs(1));
    assert!(matches!(outcome.outcome, McpCheckOutcome::Passed { tools: 2, resources: 0, prompts: 0 }));
    assert_eq!(outcome.protocol_version.as_deref(), Some("2025-06-18"));
    assert_eq!(server.calls.iter().map(|call| call["method"].as_str().unwrap()).collect::<Vec<_>>(),
        ["initialize", "notifications/initialized", "tools/list", "tools/list"]);
    assert_eq!(server.calls[3]["params"]["cursor"], "page-2");
    assert!(server.calls.iter().all(|call| call["params"].get("_meta").is_none()));
}

#[test]
fn unsupported_version_stops_without_discovery_or_fallback() {
    let mut server = server("2024-11-05", json!({}));
    let outcome = probe(&mut server, &deadline(), Duration::from_secs(1));
    assert!(matches!(outcome.outcome, McpCheckOutcome::Failed { .. }));
    assert_eq!(server.calls.len(), 1);
}

#[test]
fn absent_optional_capabilities_require_no_directory_calls() {
    let mut server = server("2025-11-25", json!({}));
    let outcome = probe(&mut server, &deadline(), Duration::from_secs(1));
    assert!(matches!(outcome.outcome, McpCheckOutcome::Passed { tools: 0, resources: 0, prompts: 0 }));
    assert_eq!(server.calls.len(), 2);
}

#[test]
fn a_declared_directory_that_rejects_listing_is_partial_not_passed() {
    let mut server = server("2025-11-25", json!({ "tools": {} }));
    server.reject_tools = true;
    assert!(matches!(probe(&mut server, &deadline(), Duration::from_secs(1)).outcome,
        McpCheckOutcome::Partial { .. }));
}

#[test]
fn server_request_with_the_same_id_is_not_a_client_response() {
    let ping = json!({ "jsonrpc": "2.0", "id": 1, "method": "ping" });
    assert!(!is_response(&ping, &json!(1)));
    assert_eq!(server_request_reply(&ping).unwrap()["result"], json!({}));
}
