use asb_core::contracts::UsageHistoryMetric;
use chrono::{DateTime, FixedOffset, SecondsFormat, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct HistoryImport {
    pub providers: Vec<ProviderHistoryPoint>,
    pub official: Vec<OfficialHistoryPoint>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct ProviderHistoryPoint {
    pub profile_id: String,
    pub query_digest: String,
    pub at: String,
    pub plan_name: Option<String>,
    pub unit: Option<String>,
    pub remaining: Option<f64>,
    pub used: Option<f64>,
    pub total: Option<f64>,
}
impl ProviderHistoryPoint {
    pub(super) fn validate(&self) -> Result<(), String> {
        if self.profile_id.is_empty() || self.query_digest.is_empty() { return Err("供应商历史身份无效".into()); }
        normalized_timestamp(&self.at)?;
        for value in [self.remaining, self.used, self.total] { validate_optional_number(value)?; }
        if self.remaining.is_none() && self.used.is_none() && self.total.is_none() { return Err("供应商历史读数为空".into()); }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct OfficialHistoryPoint {
    pub at: String,
    pub window_label: String,
    pub used_percent: f64,
    pub resets_at: Option<String>,
}
impl OfficialHistoryPoint {
    pub(super) fn validate(&self) -> Result<(), String> {
        normalized_timestamp(&self.at)?;
        if self.window_label.trim().is_empty() || !self.used_percent.is_finite()
            || !(0.0..=100.0).contains(&self.used_percent) { return Err("官方额度历史窗口无效".into()); }
        if let Some(at) = &self.resets_at { normalized_timestamp(at)?; }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ProviderSeriesKey {
    pub plan_name: Option<String>,
    pub unit: Option<String>,
    pub metric: UsageHistoryMetric,
}

pub(super) fn normalized_timestamp(value: &str) -> Result<String, String> {
    DateTime::<FixedOffset>::parse_from_rfc3339(value).map_err(|_| "历史读取时间无效".to_string())
        .map(|at| at.with_timezone(&Utc).to_rfc3339_opts(SecondsFormat::Millis, true))
}
pub(super) fn validate_optional_number(value: Option<f64>) -> Result<(), String> {
    if value.is_some_and(|value| !value.is_finite()) { return Err("供应商历史数值无效".into()); }
    Ok(())
}
pub(super) fn normalize_text(value: Option<&str>) -> Option<String> {
    value.map(str::trim).filter(|text| !text.is_empty()).map(str::to_string)
}
