//! Times cross the FFI as formatted strings, so jiff stays out of the Swift API.

use crate::ToolError;
use jiff::Timestamp;
use jiff::tz::TimeZone;
use toolbox_core::{cron, timestamp};

/// A ready-made expression for the examples menu, e.g. "0 9 * * 1-5", "Weekdays at 09:00".
#[derive(uniffi::Record)]
pub struct CronExample {
    pub pattern: String,
    pub label: String,
}

#[uniffi::export]
pub fn cron_examples() -> Vec<CronExample> {
    cron::EXAMPLES
        .iter()
        .map(|&(pattern, label)| CronExample {
            pattern: pattern.into(),
            label: label.into(),
        })
        .collect()
}

#[derive(Debug, uniffi::Record)]
pub struct CronRun {
    /// e.g. "Thu 2024-05-16 09:00:00 CEST".
    pub time: String,
    /// e.g. "in 22 hours".
    pub relative: String,
}

#[derive(Debug, uniffi::Record)]
pub struct CronExplanation {
    /// The schedule in plain English.
    pub description: String,
    /// The next runs after now, in the requested zone. Empty if it never runs again.
    pub next_runs: Vec<CronRun>,
}

/// Explains `pattern` and lists its next `count` runs after now in the IANA zone `zone`
/// (UTC if the zone is unknown).
#[uniffi::export]
pub fn cron_explain(pattern: &str, zone: &str, count: u32) -> Result<CronExplanation, ToolError> {
    explain_at(pattern, zone, Timestamp::now(), count)
}

fn explain_at(
    pattern: &str,
    zone: &str,
    now: Timestamp,
    count: u32,
) -> Result<CronExplanation, ToolError> {
    let zone = TimeZone::get(zone).unwrap_or(TimeZone::UTC);
    let now = now.to_zoned(zone);
    let explanation = cron::explain(pattern, &now, count as usize).map_err(ToolError::new)?;
    Ok(CronExplanation {
        description: explanation.description,
        next_runs: explanation
            .next_runs
            .iter()
            .map(|run| CronRun {
                time: run.strftime("%a %Y-%m-%d %H:%M:%S %Z").to_string(),
                relative: timestamp::relative(run.timestamp(), now.timestamp()),
            })
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> Timestamp {
        "2024-05-15T10:07:00Z".parse().unwrap()
    }

    #[test]
    fn formats_runs_in_zone() {
        let explanation = explain_at("0 9 * * 1-5", "Europe/Paris", now(), 2).unwrap();
        assert!(!explanation.description.is_empty());
        // 10:07 UTC is 12:07 in Paris, so 09:00 Paris time has passed today.
        assert_eq!(
            explanation.next_runs[0].time,
            "Thu 2024-05-16 09:00:00 CEST"
        );
        assert_eq!(explanation.next_runs[0].relative, "in 20 hours");
        assert_eq!(
            explanation.next_runs[1].time,
            "Fri 2024-05-17 09:00:00 CEST"
        );
    }

    #[test]
    fn unknown_zone_falls_back_to_utc() {
        let explanation = explain_at("@daily", "Nowhere/Special", now(), 1).unwrap();
        assert_eq!(explanation.next_runs[0].time, "Thu 2024-05-16 00:00:00 UTC");
    }

    #[test]
    fn invalid_patterns_are_errors() {
        let error = explain_at("61 * * * *", "UTC", now(), 1).unwrap_err();
        assert!(error.to_string().starts_with("Invalid cron expression"));
    }

    #[test]
    fn examples_are_listed() {
        assert!(
            cron_examples()
                .iter()
                .any(|example| example.pattern == "0 9 * * 1-5")
        );
    }
}
