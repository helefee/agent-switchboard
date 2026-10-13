use super::*;

fn normalize(value: serde_json::Value) -> Result<UsageReading, String> {
    normalize_reading(serde_json::from_value(value).unwrap())
}

#[test]
fn balances_and_consumption_determine_availability_without_provider_information() {
    for (input, expected) in [
        (serde_json::json!({ "remaining": 12 }), UsageAvailability::Available),
        (serde_json::json!({ "remaining": 0 }), UsageAvailability::Exhausted),
        (serde_json::json!({ "remaining": -1.25 }), UsageAvailability::Exhausted),
        (serde_json::json!({ "remaining": 0, "isValid": true }), UsageAvailability::Exhausted),
        (serde_json::json!({ "used": 100, "total": 100 }), UsageAvailability::Exhausted),
        (serde_json::json!({ "used": 120, "total": 100 }), UsageAvailability::Exhausted),
        (serde_json::json!({ "used": 0, "total": 0 }), UsageAvailability::Exhausted),
        (serde_json::json!({ "used": 10 }), UsageAvailability::Unknown),
        (serde_json::json!({ "used": 10, "isValid": true }), UsageAvailability::Available),
        (serde_json::json!({ "remaining": 12, "isValid": false, "invalidMessage": "expired" }),
            UsageAvailability::Unavailable { message: Some("expired".into()) }),
        (serde_json::json!({ "isValid": false }), UsageAvailability::Unavailable { message: None }),
    ] {
        assert_eq!(normalize(input).unwrap().availability, expected);
    }
    let reading = normalize(serde_json::json!({ "remaining": -0.0, "used": 4 })).unwrap();
    assert!(reading.remaining.unwrap().is_sign_positive());
    assert!(normalize(serde_json::json!({ "used": 4 })).unwrap().remaining.is_none());
}

#[test]
fn missing_invalid_and_non_finite_facts_are_data_errors() {
    for value in [serde_json::json!({}), serde_json::json!({ "isValid": true }),
        serde_json::json!({ "remaining": 1, "invalidMessage": "expired" }),
        serde_json::json!({ "remaining": 1, "resetsAt": "tomorrow" })] {
        assert!(normalize(value).is_err());
    }
    let mut input: UsageReadingInput = serde_json::from_value(serde_json::json!({ "remaining": 1 })).unwrap();
    input.remaining = Some(f64::NAN);
    assert!(normalize_reading(input).is_err());
    assert!(normalize_readings(vec![], "2026-10-13T00:00:00Z".into()).is_err());
}

#[test]
fn request_failures_are_classified_before_json_or_extraction() {
    for status in [401, 403, 402, 429, 500] {
        assert!(parse_response(status, "not JSON").unwrap_err().contains(&format!("HTTP {status}")));
    }
    assert!(parse_response(200, "not JSON").is_err());
    assert_eq!(parse_response(200, "{}").unwrap(), serde_json::json!({}));
}

#[test]
fn normalized_wire_shape_has_one_state_owner_and_rejects_previous_cache_readings() {
    let reading = normalize(serde_json::json!({ "remaining": 0, "isValid": true })).unwrap();
    let value = serde_json::to_value(&reading).unwrap();
    assert_eq!(value["availability"]["kind"], "exhausted");
    assert!(value.get("isValid").is_none());
    assert!(value.get("invalidMessage").is_none());
    assert_eq!(serde_json::from_value::<UsageReading>(value.clone()).unwrap(), reading);
    assert!(serde_json::from_value::<UsageReadingInput>(value).is_err());
    assert!(serde_json::from_value::<UsageReading>(serde_json::json!({
        "remaining": 0, "used": null, "total": null, "unit": "credits", "isValid": false
    })).is_err());
}

#[test]
fn declarative_fields_use_the_same_availability_rules_as_script_facts() {
    for remaining in [0.0, -1.25] {
        let summary = super::super::declarative::extract_declarative_summary(
            &serde_json::json!({ "remaining": remaining }), Some("remaining"),
            None, None, Some("credits".into()), "2026-10-13T00:00:00Z".into()).unwrap();
        assert_eq!(summary.readings[0].availability, UsageAvailability::Exhausted);
        assert_eq!(summary.readings[0].remaining, Some(remaining));
    }
}
