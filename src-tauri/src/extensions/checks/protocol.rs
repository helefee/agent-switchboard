use serde_json::{json, Value};
use super::{CheckDeadline, CheckError, McpCheckOutcome, ProbeOutcome, MAX_PAGES, SUPPORTED_PROTOCOL_VERSIONS};

pub(super) trait Connection {
    fn request(&mut self, request: &Value, deadline: &CheckDeadline) -> Result<Value, CheckError>;
    fn notify(&mut self, notification: &Value, deadline: &CheckDeadline) -> Result<(), CheckError>;
    fn set_protocol_version(&mut self, version: &str);
}

pub(super) fn probe(
    connection: &mut impl Connection,
    deadline: &CheckDeadline,
    initialize_timeout: std::time::Duration,
) -> ProbeOutcome {
    let initialize = json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": {
            "protocolVersion": SUPPORTED_PROTOCOL_VERSIONS[0], "capabilities": {},
            "clientInfo": { "name": "agent-switchboard", "version": env!("CARGO_PKG_VERSION") },
        },
    });
    let response = match connection.request(&initialize, &deadline.limited(initialize_timeout)) {
        Ok(response) => response,
        Err(error) => return ProbeOutcome::for_error(None, error),
    };
    let (version, capabilities) = match handshake(&response) {
        Ok(handshake) => handshake,
        Err(error) => return ProbeOutcome::for_error(None, error),
    };
    connection.set_protocol_version(version);
    if let Err(error) = connection.notify(
        &json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }), deadline,
    ) {
        return ProbeOutcome::for_error(Some(version.to_string()), error);
    }
    let mut counts = [0u64; 3];
    for (index, capability) in ["tools", "resources", "prompts"].iter().enumerate() {
        if !capabilities.contains_key(*capability) { continue; }
        match list_directory(connection, capability, deadline, &mut counts[index]) {
            Ok(()) => {}
            Err(error) => return ProbeOutcome::directory_error(version, error),
        }
    }
    ProbeOutcome {
        outcome: McpCheckOutcome::Passed { tools: counts[0], resources: counts[1], prompts: counts[2] },
        protocol_version: Some(version.to_string()), truncated: false,
    }
}

fn handshake(response: &Value) -> Result<(&str, &serde_json::Map<String, Value>), CheckError> {
    let result = rpc_result(response)?;
    let version = result.get("protocolVersion").and_then(Value::as_str)
        .filter(|version| SUPPORTED_PROTOCOL_VERSIONS.contains(version))
        .ok_or_else(|| CheckError::protocol("服务返回了未支持或缺失的 MCP 协议版本"))?;
    let capabilities = result.get("capabilities").and_then(Value::as_object)
        .ok_or_else(|| CheckError::protocol("初始化响应缺少能力声明"))?;
    for capability in ["tools", "resources", "prompts"] {
        if capabilities.get(capability).is_some_and(|value| !value.is_object()) {
            return Err(CheckError::protocol("服务能力声明格式无效"));
        }
    }
    let server = result.get("serverInfo");
    if !["name", "version"].iter().all(|field| server.and_then(|info| info.get(field))
        .and_then(Value::as_str).is_some()) {
        return Err(CheckError::protocol("初始化响应缺少服务标识"));
    }
    Ok((version, capabilities))
}

fn list_directory(
    connection: &mut impl Connection, capability: &str,
    deadline: &CheckDeadline, count: &mut u64,
) -> Result<(), CheckError> {
    let mut cursor: Option<String> = None;
    let mut seen = std::collections::BTreeSet::new();
    for page in 0..MAX_PAGES {
        let params = cursor.as_ref().map(|cursor| json!({ "cursor": cursor })).unwrap_or(json!({}));
        let response = connection.request(&json!({
            "jsonrpc": "2.0", "id": format!("{capability}-{page}"),
            "method": format!("{capability}/list"), "params": params,
        }), deadline)?;
        let result = rpc_result(&response)?;
        let items = result.get(capability).and_then(Value::as_array)
            .ok_or_else(|| CheckError::protocol("目录响应缺少有效的条目数组"))?;
        *count += items.len() as u64;
        match result.get("nextCursor") {
            None => return Ok(()),
            Some(value) => {
                let next = value.as_str().filter(|cursor| !cursor.is_empty())
                    .ok_or_else(|| CheckError::protocol("目录分页游标无效"))?;
                if !seen.insert(next.to_string()) {
                    return Err(CheckError::protocol("目录分页游标重复"));
                }
                cursor = Some(next.to_string());
            }
        }
    }
    Err(CheckError::output_limit())
}

pub(super) fn is_response(value: &Value, id: &Value) -> bool {
    value.get("jsonrpc").and_then(Value::as_str) == Some("2.0")
        && value.get("id") == Some(id) && value.get("method").is_none()
        && (value.get("result").is_some() ^ value.get("error").is_some())
}

pub(super) fn server_request_reply(value: &Value) -> Option<Value> {
    if value.get("jsonrpc").and_then(Value::as_str) != Some("2.0") { return None; }
    let id = value.get("id")?;
    let method = value.get("method")?.as_str()?;
    Some(if method == "ping" {
        json!({ "jsonrpc": "2.0", "id": id, "result": {} })
    } else {
        json!({ "jsonrpc": "2.0", "id": id,
            "error": { "code": -32601, "message": "Client method not supported" } })
    })
}

fn rpc_result(response: &Value) -> Result<&Value, CheckError> {
    if let Some(error) = response.get("error") {
        return Err(CheckError::protocol(format!("服务拒绝请求（JSON-RPC {}）",
            error.get("code").and_then(Value::as_i64).unwrap_or(-32603))));
    }
    response.get("result").filter(|value| value.is_object())
        .ok_or_else(|| CheckError::protocol("服务响应缺少有效的 result"))
}

#[cfg(test)]
mod tests;
