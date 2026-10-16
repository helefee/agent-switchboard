use super::data::ProviderSeriesKey;
use super::store;
use crate::local_state::LocalState;
use asb_core::contracts::{ProviderProfile, UsageHistoryMetric, UsageHistoryPoint, UsageHistorySeries};
use std::collections::BTreeMap;

const DISPLAY_POINTS: u64 = 720;

struct Sample {
    total: u64,
    index: u64,
    first: String,
    last: String,
    points: Vec<UsageHistoryPoint>,
}
impl Sample {
    fn new(total: u64) -> Self {
        Self { total, index: 0, first: String::new(), last: String::new(), points: Vec::new() }
    }
    fn push(&mut self, at: &str, value: f64) {
        if self.index == 0 { self.first = at.to_string(); }
        self.last = at.to_string();
        let keep = self.total <= DISPLAY_POINTS || self.index == 0 || self.index + 1 == self.total
            || (self.index as u128 * (DISPLAY_POINTS - 1) as u128 / (self.total - 1) as u128)
                != ((self.index - 1) as u128 * (DISPLAY_POINTS - 1) as u128 / (self.total - 1) as u128);
        if keep { self.points.push(UsageHistoryPoint { at: at.to_string(), value }); }
        self.index += 1;
    }
    fn finish(self, id: String, label: String, unit: Option<String>, metric: UsageHistoryMetric) -> UsageHistorySeries {
        UsageHistorySeries { id, label, unit, metric, points: self.points,
            total_points: self.total, first_at: self.first, last_at: self.last }
    }
}

pub(crate) fn provider_series(state: &LocalState, profile: &ProviderProfile) -> Result<Vec<UsageHistorySeries>, String> {
    let Some(query) = &profile.usage_query else { return Ok(Vec::new()); };
    let digest = crate::usage_cache::query_digest(query)?;
    let connection = store::read(state)?;
    connection.execute_batch("BEGIN DEFERRED").map_err(|error| error.to_string())?;
    let mut groups = BTreeMap::new();
    {
        let mut counts = connection.prepare("SELECT plan_name,unit,COUNT(remaining), \
            SUM(CASE WHEN used IS NOT NULL OR (total IS NOT NULL AND remaining IS NOT NULL AND total>=remaining) THEN 1 ELSE 0 END) \
            FROM provider_points WHERE profile_id=?1 AND query_digest=?2 GROUP BY plan_name,unit").map_err(|e| e.to_string())?;
        let mut rows = counts.query([&profile.id, &digest]).map_err(|e| e.to_string())?;
        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let plan_name: Option<String> = row.get(0).map_err(|e| e.to_string())?;
            let unit: Option<String> = row.get(1).map_err(|e| e.to_string())?;
            for (column, metric) in [(2, UsageHistoryMetric::Remaining), (3, UsageHistoryMetric::Used)] {
                let total: i64 = row.get(column).map_err(|e| e.to_string())?;
                if total > 0 { groups.insert(ProviderSeriesKey { plan_name: plan_name.clone(), unit: unit.clone(), metric }, Sample::new(total as u64)); }
            }
        }
    }
    let mut statement = connection.prepare("SELECT at,plan_name,unit,remaining,used,total FROM provider_points \
        WHERE profile_id=?1 AND query_digest=?2 ORDER BY at,id").map_err(|e| e.to_string())?;
    let mut rows = statement.query([&profile.id, &digest]).map_err(|e| e.to_string())?;
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let at: String = row.get(0).map_err(|e| e.to_string())?;
        let plan_name: Option<String> = row.get(1).map_err(|e| e.to_string())?;
        let unit: Option<String> = row.get(2).map_err(|e| e.to_string())?;
        let remaining: Option<f64> = row.get(3).map_err(|e| e.to_string())?;
        let used: Option<f64> = row.get(4).map_err(|e| e.to_string())?;
        let total: Option<f64> = row.get(5).map_err(|e| e.to_string())?;
        let used = used.or_else(|| total.zip(remaining).filter(|(t, r)| t >= r).map(|(t, r)| t - r));
        for (metric, value) in [(UsageHistoryMetric::Remaining, remaining), (UsageHistoryMetric::Used, used)] {
            if let Some(value) = value {
                let key = ProviderSeriesKey { plan_name: plan_name.clone(), unit: unit.clone(), metric };
                if let Some(sample) = groups.get_mut(&key) { sample.push(&at, value); }
            }
        }
    }
    Ok(groups.into_iter().map(|(key, sample)| {
        let plan = key.plan_name.as_deref().unwrap_or("默认方案");
        let metric = match key.metric { UsageHistoryMetric::Remaining => "余额", UsageHistoryMetric::Used => "已用", UsageHistoryMetric::UsedPercent => "已用比例" };
        let id = series_id(&[&profile.id, &digest, plan, key.unit.as_deref().unwrap_or(""), metric]);
        sample.finish(id, format!("{plan}{metric}"), key.unit, key.metric)
    }).collect())
}

pub(crate) fn official_series(state: &LocalState) -> Result<Vec<UsageHistorySeries>, String> {
    let connection = store::read(state)?;
    connection.execute_batch("BEGIN DEFERRED").map_err(|e| e.to_string())?;
    let generation = store::generation(&connection)?;
    let mut groups = BTreeMap::new();
    let mut counts = connection.prepare("SELECT window_label,COUNT(*) FROM official_points WHERE generation=?1 GROUP BY window_label")
        .map_err(|e| e.to_string())?;
    let mut rows = counts.query([generation]).map_err(|e| e.to_string())?;
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        groups.insert(row.get::<_, String>(0).map_err(|e| e.to_string())?, Sample::new(row.get::<_, i64>(1).map_err(|e| e.to_string())? as u64));
    }
    let mut statement = connection.prepare("SELECT window_label,at,used_percent FROM official_points WHERE generation=?1 ORDER BY at,id")
        .map_err(|e| e.to_string())?;
    let mut rows = statement.query([generation]).map_err(|e| e.to_string())?;
    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let label: String = row.get(0).map_err(|e| e.to_string())?;
        if let Some(sample) = groups.get_mut(&label) {
            sample.push(&row.get::<_, String>(1).map_err(|e| e.to_string())?, row.get(2).map_err(|e| e.to_string())?);
        }
    }
    Ok(groups.into_iter().map(|(label, sample)|
        sample.finish(series_id(&["official", &generation.to_string(), &label]), label,
            Some("%".into()), UsageHistoryMetric::UsedPercent)).collect())
}
fn series_id(parts: &[&str]) -> String { asb_switch::sha256_hex(&parts.join("\u{1f}"))[..16].to_string() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_sampling_covers_the_full_span_and_keeps_exact_first_and_last_points() {
        let mut sample = Sample::new(10_000);
        for index in 0..10_000 { sample.push(&format!("time-{index:05}"), index as f64); }
        let series = sample.finish("id".into(), "label".into(), None, UsageHistoryMetric::Used);
        assert_eq!(series.total_points, 10_000);
        assert_eq!(series.points.len(), DISPLAY_POINTS as usize);
        assert_eq!(series.first_at, "time-00000");
        assert_eq!(series.last_at, "time-09999");
        assert_eq!(series.points[0].value, 0.0);
        assert_eq!(series.points.last().unwrap().value, 9_999.0);
    }
}
