use serde_json::{Map, Value};

pub(super) fn content(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(items) => items.iter().map(content).filter(|text| !text.is_empty())
            .collect::<Vec<_>>().join("\n\n"),
        Value::Object(object) => block(object),
        Value::Null => String::new(),
        _ => json(value),
    }
}

fn block(object: &Map<String, Value>) -> String {
    let kind = object.get("type").and_then(Value::as_str).unwrap_or("");
    match kind {
        "text" | "input_text" | "output_text" | "summary_text" =>
            object.get("text").map(content).unwrap_or_else(|| json(&Value::Object(object.clone()))),
        "thinking" => object.get("thinking").map(content).filter(|text| !text.is_empty())
            .unwrap_or_else(|| json(&Value::Object(object.clone()))),
        "tool_use" | "server_tool_use" => format!("### 工具调用 · {}\n\n{}",
            object.get("name").and_then(Value::as_str).unwrap_or("未知工具"),
            json(&Value::Object(object.clone()))),
        "tool_result" => format!("### 工具结果\n\n{}", json(&Value::Object(object.clone()))),
        "image" | "input_image" | "image_url" => image(object),
        "encrypted_content" | "redacted_thinking" =>
            format!("加密内容：原生记录未提供可解密正文。\n\n{}", json(&Value::Object(object.clone()))),
        _ if kind.is_empty() && object.len() == 1 =>
            object.get("text").or_else(|| object.get("content")).or_else(|| object.get("value"))
                .map(content).unwrap_or_else(|| json(&Value::Object(object.clone()))),
        _ => json(&Value::Object(object.clone())),
    }
}

fn image(object: &Map<String, Value>) -> String {
    let url = object.get("image_url").or_else(|| object.get("url"))
        .and_then(|value| value.as_str().or_else(|| value.get("url").and_then(Value::as_str)));
    let source = object.get("source");
    let encoded = source.and_then(|value| value.get("data")).and_then(Value::as_str)
        .zip(source.and_then(|value| value.get("media_type")).and_then(Value::as_str))
        .map(|(data, mime)| format!("data:{mime};base64,{data}"));
    match url.map(str::to_string).or(encoded) {
        Some(url) if url.starts_with("data:image/") => "图片附件：展开消息查看原图。".into(),
        _ => format!("### 图片附件\n\n{}", json(&Value::Object(object.clone()))),
    }
}

pub(super) fn json(value: &Value) -> String {
    let text = serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string());
    let longest = text.split(|character| character != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat(longest.max(2) + 1);
    format!("{fence}json\n{text}\n{fence}")
}

pub(super) fn record(value: &Value) -> Option<(String, String)> {
    let envelope = if value.get("type").and_then(Value::as_str) == Some("event_msg") {
        let payload = value.get("payload")?;
        if !matches!(payload.get("type").and_then(Value::as_str), Some("item_completed" | "item_started")) {
            return None;
        }
        payload.get("item")?
    } else if value.get("type").and_then(Value::as_str) == Some("response_item") {
        value.get("payload")?
    } else { value };
    let kind = envelope.get("type").and_then(Value::as_str).unwrap_or("");
    if matches!(kind, "session_meta" | "turn_context" | "token_usage_record" | "event_msg" | "progress"
        | "file-history-snapshot" | "queue-operation" | "custom-title") { return None; }
    let message = envelope.get("message").filter(|message| message.is_object()).unwrap_or(envelope);
    let role = message.get("role").and_then(Value::as_str).map(str::to_string)
        .unwrap_or_else(|| role(kind).to_string());
    let body = match kind {
        "message" | "user" | "assistant" | "system" =>
            message.get("content").map(content).unwrap_or_else(|| json(envelope)),
        "userMessage" => envelope.get("content").map(content).unwrap_or_else(|| json(envelope)),
        "agentMessage" => {
            let mut parts = vec![envelope.get("text").map(content).unwrap_or_default()];
            for name in ["memoryCitation", "questions"] {
                if let Some(value) = envelope.get(name).filter(|value| nonempty(value)) {
                    parts.push(json(&serde_json::json!({name:value})));
                }
            }
            parts.into_iter().filter(|part| !part.is_empty()).collect::<Vec<_>>().join("\n\n")
        },
        "imageGeneration" | "image_generation_call" if super::images::generated_image(envelope.as_object()?).is_some() => {
            let mut metadata = envelope.clone();
            metadata["result"] = Value::String("图片附件：展开消息查看原图。".into());
            json(&metadata)
        },
        "agent_message" => format!("### 代理消息 · {}\n\n{}",
            envelope.get("author").and_then(Value::as_str).unwrap_or("未知代理"),
            envelope.get("content").map(content).unwrap_or_else(|| json(envelope))),
        "function_call_output" | "custom_tool_call_output" =>
            envelope.get("output").map(content).unwrap_or_else(|| json(envelope)),
        "reasoning" if envelope.get("encrypted_content").is_some_and(nonempty) =>
            format!("加密推理：原生记录未提供可解密正文。\n\n{}", json(envelope)),
        _ if kind.is_empty() && message.get("content").is_some() => content(&message["content"]),
        _ => json(envelope),
    };
    Some((role, if body.trim().is_empty() { json(envelope) } else { body }))
}

fn nonempty(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Array(items) => !items.is_empty(),
        Value::Object(object) => !object.is_empty(),
        Value::String(text) => !text.is_empty(),
        _ => true,
    }
}

fn role(kind: &str) -> &'static str {
    match kind {
        "user" | "userMessage" => "user",
        "assistant" | "agentMessage" | "agent_message" => "assistant",
        "system" | "contextCompaction" | "compaction" | "plan" => "system",
        "reasoning" => "reasoning",
        _ => "tool",
    }
}
