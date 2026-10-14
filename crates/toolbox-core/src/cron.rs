//! Cron expression explainer: a plain-English description and the next run times.

use croner::Cron;
use jiff::Zoned;
use std::str::FromStr;

pub const EXAMPLES: &[(&str, &str)] = &[
    ("*/15 * * * *", "Every 15 minutes"),
    ("0 * * * *", "Every hour"),
    ("30 2 * * *", "Daily at 02:30"),
    ("0 9 * * 1-5", "Weekdays at 09:00"),
    ("0 0 * * 0", "Sundays at midnight"),
    ("0 0 1 * *", "Monthly on the 1st"),
    ("0 0 1 1 *", "Yearly on January 1st"),
];

/// The most upcoming runs [`explain`] lists, so a huge count can't take forever.
pub const MAX_RUNS: usize = 100;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Explanation {
    pub description: String,
    /// Upcoming run times after `now`, in its time zone.
    pub next_runs: Vec<Zoned>,
}

/// Explains a cron pattern: 5 fields (minute hour day month weekday), an optional leading
/// seconds field, or a nickname such as `@daily`.
pub fn explain(pattern: &str, now: &Zoned, count: usize) -> Result<Explanation, String> {
    let pattern = pattern.trim();
    if pattern.is_empty() {
        return Err("Enter a cron expression, e.g. */15 * * * *".into());
    }
    let cron =
        Cron::from_str(pattern).map_err(|error| format!("Invalid cron expression: {error}"))?;
    let next_runs = cron
        .iter_after(now.clone())
        .take(count.min(MAX_RUNS))
        .collect();
    Ok(Explanation {
        description: cron.describe(),
        next_runs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::Weekday;

    fn now() -> Zoned {
        "2024-05-15T10:07:00[UTC]".parse().unwrap()
    }

    #[test]
    fn run_counts_are_capped() {
        let explanation = explain("* * * * * *", &now(), usize::MAX).unwrap();
        assert_eq!(explanation.next_runs.len(), MAX_RUNS);
    }

    #[test]
    fn impossible_dates_have_no_runs() {
        for pattern in ["0 0 30 2 *", "0 0 31 4,6,9,11 *"] {
            assert!(explain(pattern, &now(), 5).unwrap().next_runs.is_empty());
        }
    }

    #[test]
    fn describes_and_schedules() {
        let explanation = explain("0 9 * * 1-5", &now(), 5).unwrap();
        assert!(
            explanation.description.contains("09:00") || explanation.description.contains("9:00"),
            "{}",
            explanation.description
        );
        assert_eq!(explanation.next_runs.len(), 5);
        for run in &explanation.next_runs {
            assert_eq!((run.hour(), run.minute()), (9, 0));
            assert!(
                !matches!(run.weekday(), Weekday::Saturday | Weekday::Sunday),
                "{run}"
            );
        }
        // 2024-05-15 is a Wednesday, already past 09:00, so the first run is Thursday.
        assert_eq!(
            explanation.next_runs[0].to_string(),
            "2024-05-16T09:00:00+00:00[UTC]"
        );
    }

    #[test]
    fn steps_and_nicknames() {
        let runs = explain("*/15 * * * *", &now(), 3).unwrap().next_runs;
        let minutes: Vec<i8> = runs.iter().map(|run| run.minute()).collect();
        assert_eq!(minutes, [15, 30, 45]);
        let daily = explain("@daily", &now(), 1).unwrap();
        assert_eq!(
            daily.next_runs[0].to_string(),
            "2024-05-16T00:00:00+00:00[UTC]"
        );
    }

    #[test]
    fn rejects_invalid_patterns() {
        assert!(explain("", &now(), 1).is_err());
        assert!(
            explain("61 * * * *", &now(), 1)
                .unwrap_err()
                .starts_with("Invalid cron expression")
        );
        assert!(explain("not cron", &now(), 1).is_err());
    }

    #[test]
    fn examples_are_valid() {
        for (pattern, _) in EXAMPLES {
            assert!(explain(pattern, &now(), 1).is_ok(), "{pattern}");
        }
    }
}
