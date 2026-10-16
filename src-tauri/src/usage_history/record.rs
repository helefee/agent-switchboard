use super::data::{normalize_text, normalized_timestamp, validate_optional_number, OfficialHistoryPoint, ProviderHistoryPoint};
use super::store;
use crate::local_state::LocalState;
use asb_core::contracts::{CodexOfficialQuota, ProviderProfile, UsageSummary};

/// Records every numeric fact in one successful provider network query.
/// The caller treats persistence errors as warnings so a fresh query result
/// remains visible even when this optional local history cannot be updated.
pub(crate) fn record_provider(
    state: &LocalState,
    profile: &ProviderProfile,
    summary: &UsageSummary,
) -> Result<(), String> {
    let query = profile
        .usage_query
        .as_ref()
        .ok_or_else(|| "该供应商尚未配置用量查询".to_string())?;
    let digest = crate::usage_cache::query_digest(query)?;
    let points = provider_points(&profile.id, &digest, summary)?;
    if points.is_empty() { return Ok(()); }
    store::append_provider(state, &points)
}

fn provider_points(profile_id: &str, digest: &str, summary: &UsageSummary) -> Result<Vec<ProviderHistoryPoint>, String> {
    let at = normalized_timestamp(&summary.at)?;
    summary
        .readings
        .iter()
        .filter(|reading| reading.remaining.is_some() || reading.used.is_some() || reading.total.is_some())
        .map(|reading| {
            validate_optional_number(reading.remaining)?;
            validate_optional_number(reading.used)?;
            validate_optional_number(reading.total)?;
            Ok(ProviderHistoryPoint {
                profile_id: profile_id.to_string(),
                query_digest: digest.to_string(),
                at: at.clone(),
                plan_name: normalize_text(reading.plan_name.as_deref()),
                unit: normalize_text(reading.unit.as_deref()),
                remaining: reading.remaining,
                used: reading.used,
                total: reading.total,
            })
        })
        .collect::<Result<Vec<_>, String>>()

}

/// Records every server-declared official quota window from a successful
/// official read. `account_changed` comes from the existing comparison
/// baseline and prevents points belonging to two detected accounts from
/// sharing one trend.
pub(crate) fn record_official(
    state: &LocalState,
    quota: &CodexOfficialQuota,
    account_changed: bool,
) -> Result<(), String> {
    let at = quota
        .at
        .as_deref()
        .ok_or_else(|| "官方额度读取时间缺失".to_string())
        .and_then(normalized_timestamp)?;
    let points = quota
        .windows
        .iter()
        .map(|window| {
            let point = OfficialHistoryPoint {
                at: at.clone(),
                window_label: window.label.trim().to_string(),
                used_percent: window.used_percent,
                resets_at: window
                    .resets_at
                    .as_deref()
                    .map(normalized_timestamp)
                    .transpose()?,
            };
            point.validate()?;
            Ok(point)
        })
        .collect::<Result<Vec<_>, String>>()?;

    store::append_official(state, account_changed, &points)
}

#[cfg(test)]
mod tests {
    use super::*;
    use asb_core::contracts::{UsageAvailability, UsageReading};

    #[test]
    fn a_reading_without_numbers_does_not_discard_other_successful_numeric_facts() {
        let reading = |remaining, availability| UsageReading { plan_name: None, remaining, used: None, total: None,
            unit: Some("USD".into()), resets_at: None, availability, extra: None };
        let summary = UsageSummary { at: "2026-10-16T00:00:00Z".into(), readings: vec![
            reading(None, UsageAvailability::Unknown), reading(Some(12.0), UsageAvailability::Unavailable { message: None }),
            reading(Some(3.0), UsageAvailability::Available)] };
        let points = provider_points("provider", "query", &summary).unwrap();
        assert_eq!(points.len(), 2);
        assert_eq!(points[0].remaining, Some(12.0));
        assert_eq!(points[1].remaining, Some(3.0));
    }
}
