use asb_core::contracts::{UsageAvailability, UsageReading, UsageReadingInput, UsageSummary};

pub(super) fn validate_http_status(status: u16) -> Result<(), String> {
    if status == 401 || status == 403 {
        return Err(format!("服务地址拒绝了 API 密钥（HTTP {status}），请确认密钥仍然有效"));
    }
    if !(200..300).contains(&status) {
        return Err(format!("用量查询返回 HTTP {status}"));
    }
    Ok(())
}

pub(super) fn parse_response(status: u16, body: &str) -> Result<serde_json::Value, String> {
    validate_http_status(status)?;
    serde_json::from_str(body).map_err(|_| "用量响应不是有效 JSON".to_string())
}

pub(super) fn normalize_readings(
    readings: Vec<UsageReadingInput>,
    at: String,
) -> Result<UsageSummary, String> {
    if readings.is_empty() {
        return Err("用量查询没有返回读数".into());
    }
    let readings = readings.into_iter().map(normalize_reading).collect::<Result<_, _>>()?;
    Ok(UsageSummary { readings, at })
}

fn normalize_reading(input: UsageReadingInput) -> Result<UsageReading, String> {
    let numbers = [input.remaining, input.used, input.total];
    if numbers.iter().flatten().any(|number| !number.is_finite()) {
        return Err("用量读数必须是有限数值".into());
    }
    if numbers.iter().all(Option::is_none) && input.is_valid != Some(false) {
        return Err("每组用量读数至少要返回一个数值或明确的不可用状态".into());
    }
    if input.resets_at.as_ref().is_some_and(|at| chrono::DateTime::parse_from_rfc3339(at).is_err()) {
        return Err("用量读数 resetsAt 必须是含时区的 RFC 3339 时间".into());
    }
    if input.invalid_message.is_some() && input.is_valid != Some(false) {
        return Err("用量读数 invalidMessage 只能用于 isValid 为 false 的输入".into());
    }
    let availability = if input.is_valid == Some(false) {
        UsageAvailability::Unavailable { message: input.invalid_message }
    } else if input.remaining.is_some_and(|remaining| remaining <= 0.0)
        || (input.remaining.is_none()
            && input.used.zip(input.total).is_some_and(|(used, total)| total >= 0.0 && used >= total))
    {
        UsageAvailability::Exhausted
    } else if input.remaining.is_some_and(|remaining| remaining > 0.0) || input.is_valid == Some(true) {
        UsageAvailability::Available
    } else {
        UsageAvailability::Unknown
    };
    let normalize_zero = |number: f64| if number == 0.0 { 0.0 } else { number };
    Ok(UsageReading {
        plan_name: input.plan_name,
        remaining: input.remaining.map(normalize_zero),
        used: input.used.map(normalize_zero),
        total: input.total.map(normalize_zero),
        unit: input.unit,
        resets_at: input.resets_at,
        availability,
        extra: input.extra,
    })
}

#[cfg(test)]
mod tests;
