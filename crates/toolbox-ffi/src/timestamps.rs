//! Timestamp conversion. Times cross the FFI as strings and integers, and time zones as IANA
//! names.
//!
//! This crate doesn't depend on jiff, so jiff's `Timestamp` and `TimeZone` can't be named
//! here. Values come from toolbox-core's functions and the compiler infers their types;
//! [`current_time`] pins the type with a core function's signature.

use crate::ToolError;
use std::time::SystemTime;
use toolbox_core::timestamp::{self, Unit};

/// Zones shown on first open, after this Mac's own.
const DEFAULT_ZONES: [&str; 5] = [
    "UTC",
    "America/New_York",
    "Europe/London",
    "Asia/Kolkata",
    "Asia/Tokyo",
];

#[derive(Clone, Copy, PartialEq, Eq, Hash, uniffi::Enum)]
pub enum TimestampUnit {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
}

impl From<TimestampUnit> for Unit {
    fn from(unit: TimestampUnit) -> Self {
        match unit {
            TimestampUnit::Seconds => Unit::Seconds,
            TimestampUnit::Milliseconds => Unit::Milliseconds,
            TimestampUnit::Microseconds => Unit::Microseconds,
            TimestampUnit::Nanoseconds => Unit::Nanoseconds,
        }
    }
}

/// A labelled value, e.g. label "RFC 2822", value "Tue, 14 Nov 2023 22:13:20 +0000".
#[derive(uniffi::Record)]
pub struct TimestampValue {
    pub label: String,
    pub value: String,
}

/// The time in one zone, e.g. "2023-11-14 17:13:20 EST (-05:00)".
#[derive(uniffi::Record)]
pub struct TimestampZoneTime {
    pub zone: String,
    pub time: String,
}

#[derive(uniffi::Record)]
pub struct TimestampConversion {
    /// How the input was read, e.g. "a Unix time in seconds".
    pub source: String,
    /// Unix times, ISO 8601, RFC 2822, relative and calendar details, in display order.
    pub values: Vec<TimestampValue>,
    /// The time in each zone asked for, in the same order. Unknown zones are left out.
    pub zones: Vec<TimestampZoneTime>,
}

/// e.g. "seconds".
#[uniffi::export]
pub fn timestamps_unit_label(unit: TimestampUnit) -> String {
    Unit::from(unit).label().into()
}

/// The current Unix time in seconds.
#[uniffi::export]
pub fn timestamps_now() -> i64 {
    current_time(timestamp::unix).map_or(0, |now| {
        i64::try_from(timestamp::unix(now, Unit::Seconds)).unwrap_or_default()
    })
}

/// The zones to show on first open: this Mac's, then a few around the world.
#[uniffi::export]
pub fn timestamps_default_zones() -> Vec<String> {
    let mut names: Vec<String> = timestamp::system_zone_name().into_iter().collect();
    for name in DEFAULT_ZONES {
        if !names.iter().any(|n| n == name) {
            names.push(name.to_owned());
        }
    }
    names
}

/// Reads `input` as a Unix time (in `unit`, or guessed from its size when `None`) or a date,
/// reading dates without an offset in `input_zone`, then shows it in every format and in each
/// of `zones`.
#[uniffi::export]
pub fn timestamps_convert(
    input: &str,
    unit: Option<TimestampUnit>,
    input_zone: &str,
    zones: Vec<String>,
) -> Result<TimestampConversion, ToolError> {
    let now = current_time(timestamp::unix).ok_or_else(|| ToolError::new("No system clock"))?;
    let zone_named = |name: &str| now.in_tz(name).ok().map(|zoned| zoned.time_zone().clone());
    let input_zone = zone_named(input_zone)
        .or_else(|| zone_named("UTC"))
        .ok_or_else(|| ToolError::new("No time zone database"))?;

    let (at, source) =
        timestamp::parse(input, unit.map(Into::into), &input_zone).map_err(ToolError::new)?;
    let value = |label: &str, value: String| TimestampValue {
        label: label.into(),
        value,
    };
    let values = vec![
        value(
            "Unix seconds",
            timestamp::unix(at, Unit::Seconds).to_string(),
        ),
        value(
            "Unix milliseconds",
            timestamp::unix(at, Unit::Milliseconds).to_string(),
        ),
        value("ISO 8601 (UTC)", at.to_string()),
        value("RFC 2822", timestamp::rfc2822(at, &input_zone)),
        value("Relative", timestamp::relative(at, now)),
        value("Calendar", timestamp::calendar_details(at, &input_zone)),
    ];
    let zones = zones
        .into_iter()
        .filter_map(|name| {
            let time = timestamp::format_in_zone(at, &zone_named(&name)?);
            Some(TimestampZoneTime { zone: name, time })
        })
        .collect();
    Ok(TimestampConversion {
        source: source.to_string(),
        values,
        zones,
    })
}

/// The current time as jiff's `Timestamp`, which this crate can't name: `_unix` is
/// `timestamp::unix`, whose parameter type tells the compiler which type `T` is.
pub(crate) fn current_time<T: TryFrom<SystemTime>>(_unix: fn(T, Unit) -> i128) -> Option<T> {
    SystemTime::now().try_into().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn convert(input: &str, zone: &str, zones: &[&str]) -> TimestampConversion {
        let zones = zones.iter().map(|&zone| zone.to_owned()).collect();
        timestamps_convert(input, None, zone, zones).unwrap()
    }

    #[test]
    fn converts_in_every_format_and_zone() {
        let conversion = convert(
            "1700000000",
            "America/New_York",
            &["UTC", "Nowhere/Invalid", "Asia/Tokyo"],
        );
        assert_eq!(conversion.source, "a Unix time in seconds");
        let values: Vec<(&str, &str)> = conversion
            .values
            .iter()
            .map(|v| (v.label.as_str(), v.value.as_str()))
            .collect();
        assert_eq!(values[0], ("Unix seconds", "1700000000"));
        assert_eq!(values[1], ("Unix milliseconds", "1700000000000"));
        assert_eq!(values[2], ("ISO 8601 (UTC)", "2023-11-14T22:13:20Z"));
        assert_eq!(values[3], ("RFC 2822", "Tue, 14 Nov 2023 17:13:20 -0500"));
        assert!(values[4].1.ends_with("ago"));
        let zones: Vec<&str> = conversion.zones.iter().map(|z| z.zone.as_str()).collect();
        assert_eq!(zones, ["UTC", "Asia/Tokyo"]);
        assert_eq!(conversion.zones[1].time, "2023-11-15 07:13:20 JST (+09:00)");
    }

    #[test]
    fn reads_local_dates_in_the_input_zone() {
        let conversion = convert("2023-11-14 17:13:20", "America/New_York", &[]);
        assert_eq!(conversion.values[0].value, "1700000000");
        // An unknown input zone falls back to UTC.
        let conversion = convert("2023-11-14 22:13:20", "Nowhere/Invalid", &[]);
        assert_eq!(conversion.values[0].value, "1700000000");
    }

    #[test]
    fn forces_units_and_reports_errors() {
        let forced = timestamps_convert(
            "1700000000",
            Some(TimestampUnit::Milliseconds),
            "UTC",
            vec![],
        )
        .unwrap();
        assert_eq!(forced.values[0].value, "1700000");
        let error = timestamps_convert("next tuesday", None, "UTC", vec![]).err();
        assert!(
            error
                .unwrap()
                .to_string()
                .starts_with("Not a recognized format")
        );
    }

    #[test]
    fn default_zones_have_no_duplicates() {
        let zones = timestamps_default_zones();
        assert!(zones.contains(&"UTC".to_owned()));
        let mut unique = zones.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), zones.len());
        assert!(timestamps_now() > 1_700_000_000);
    }
}
