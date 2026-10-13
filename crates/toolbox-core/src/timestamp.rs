//! Timestamp conversion: Unix times in any unit, RFC 3339 / ISO 8601, RFC 2822 and local
//! date-times, shown in any IANA time zone.

use jiff::civil::{Date, DateTime};
use jiff::tz::TimeZone;
use jiff::{Timestamp, Zoned};
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
}

impl Unit {
    pub const ALL: [Unit; 4] = [
        Unit::Seconds,
        Unit::Milliseconds,
        Unit::Microseconds,
        Unit::Nanoseconds,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Unit::Seconds => "seconds",
            Unit::Milliseconds => "milliseconds",
            Unit::Microseconds => "microseconds",
            Unit::Nanoseconds => "nanoseconds",
        }
    }

    fn nanos(self) -> i128 {
        match self {
            Unit::Seconds => 1_000_000_000,
            Unit::Milliseconds => 1_000_000,
            Unit::Microseconds => 1_000,
            Unit::Nanoseconds => 1,
        }
    }

    /// Guesses the unit of a Unix time from its number of integer digits: up to 11 digits
    /// covers seconds until the year 5138.
    fn detect(digits: usize) -> Unit {
        match digits {
            0..=11 => Unit::Seconds,
            12..=14 => Unit::Milliseconds,
            15..=17 => Unit::Microseconds,
            _ => Unit::Nanoseconds,
        }
    }
}

/// How the input was understood.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    Now,
    Unix(Unit),
    Rfc3339,
    Rfc2822,
    /// An ISO 8601 date-time with an `[Area/City]` time zone annotation.
    ZonedIso,
    /// A date or date-time without an offset, read in the selected time zone.
    Civil,
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Source::Now => write!(f, "the current time"),
            Source::Unix(unit) => write!(f, "a Unix time in {}", unit.label()),
            Source::Rfc3339 => write!(f, "an RFC 3339 / ISO 8601 date-time"),
            Source::Rfc2822 => write!(f, "an RFC 2822 date-time"),
            Source::ZonedIso => write!(f, "an ISO 8601 date-time with a time zone"),
            Source::Civil => write!(f, "a local date-time in the selected time zone"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Empty,
    Unrecognized,
    OutOfRange,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Empty => write!(f, "Enter a timestamp or date"),
            Error::Unrecognized => write!(
                f,
                "Not a recognized format. Try 1700000000, 2023-11-14T22:13:20Z, 2023-11-14 17:30 or an RFC 2822 date"
            ),
            Error::OutOfRange => write!(
                f,
                "That time is outside the supported range (years -9999 to 9999)"
            ),
        }
    }
}

impl std::error::Error for Error {}

/// Parses `input`. `unit` forces how a plain number is read; `None` guesses from its size.
/// Dates without an offset are read in `zone`.
pub fn parse(
    input: &str,
    unit: Option<Unit>,
    zone: &TimeZone,
) -> Result<(Timestamp, Source), Error> {
    let input = input.trim();
    if input.is_empty() {
        return Err(Error::Empty);
    }
    if input.eq_ignore_ascii_case("now") {
        return Ok((Timestamp::now(), Source::Now));
    }
    if let Some(result) = parse_unix(input, unit) {
        return result;
    }
    if let Ok(timestamp) = input.parse::<Timestamp>() {
        return Ok((timestamp, Source::Rfc3339));
    }
    if let Ok(zoned) = input.parse::<Zoned>() {
        return Ok((zoned.timestamp(), Source::ZonedIso));
    }
    if let Ok(zoned) = jiff::fmt::rfc2822::parse(input) {
        return Ok((zoned.timestamp(), Source::Rfc2822));
    }
    let civil = input.parse::<DateTime>().or_else(|_| {
        input
            .parse::<Date>()
            .map(|date| date.to_datetime(jiff::civil::Time::midnight()))
    });
    if let Ok(civil) = civil {
        let zoned = civil
            .to_zoned(zone.clone())
            .map_err(|_| Error::OutOfRange)?;
        return Ok((zoned.timestamp(), Source::Civil));
    }
    Err(Error::Unrecognized)
}

/// A Unix time such as `1700000000`, `-86400`, `1_700_000_000` or `1700000000.25`.
/// Returns `None` if `input` isn't a number at all.
fn parse_unix(input: &str, unit: Option<Unit>) -> Option<Result<(Timestamp, Source), Error>> {
    let cleaned: String = input.chars().filter(|c| *c != '_' && *c != ',').collect();
    let (negative, digits) = match cleaned.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, cleaned.strip_prefix('+').unwrap_or(&cleaned)),
    };
    let (whole, fraction) = digits.split_once('.').unwrap_or((digits, ""));
    if whole.is_empty()
        || !whole.bytes().all(|b| b.is_ascii_digit())
        || !fraction.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let unit = unit.unwrap_or_else(|| Unit::detect(whole.len()));
    let result = (|| {
        let whole: i128 = whole.parse().ok()?;
        // Fractional digits beyond nanosecond precision are dropped.
        let fraction_nanos: i128 = if fraction.is_empty() {
            0
        } else {
            let padded = format!("{:0<18}", &fraction[..fraction.len().min(18)]);
            padded.parse::<i128>().ok()? * unit.nanos() / 1_000_000_000_000_000_000
        };
        let nanos = whole
            .checked_mul(unit.nanos())?
            .checked_add(fraction_nanos)?;
        let nanos = if negative { -nanos } else { nanos };
        // Checked here because jiff-core 0.1.1's `from_nanosecond` skips its own range check
        // for very large inputs (a debug assertion fires instead of returning an error).
        let range = Timestamp::MIN.as_nanosecond()..=Timestamp::MAX.as_nanosecond();
        if !range.contains(&nanos) {
            return None;
        }
        Timestamp::from_nanosecond(nanos).ok()
    })();
    Some(
        result
            .map(|timestamp| (timestamp, Source::Unix(unit)))
            .ok_or(Error::OutOfRange),
    )
}

/// The Unix time of `timestamp` in `unit`, truncated toward negative infinity for seconds,
/// milliseconds and microseconds.
pub fn unix(timestamp: Timestamp, unit: Unit) -> i128 {
    timestamp.as_nanosecond().div_euclid(unit.nanos())
}

/// e.g. "2023-11-14 17:13:20 EST (-05:00)".
pub fn format_in_zone(timestamp: Timestamp, zone: &TimeZone) -> String {
    timestamp
        .to_zoned(zone.clone())
        .strftime("%Y-%m-%d %H:%M:%S %Z (%:z)")
        .to_string()
}

/// e.g. "Tuesday · week 46 · day 318 of the year", in `zone`.
pub fn calendar_details(timestamp: Timestamp, zone: &TimeZone) -> String {
    let zoned = timestamp.to_zoned(zone.clone());
    let week = zoned.date().iso_week_date().week();
    format!(
        "{} · week {week} · day {} of the year",
        zoned.strftime("%A"),
        zoned.day_of_year()
    )
}

pub fn rfc2822(timestamp: Timestamp, zone: &TimeZone) -> String {
    jiff::fmt::rfc2822::to_string(&timestamp.to_zoned(zone.clone())).unwrap_or_default()
}

/// "in 3 hours", "2 days ago" or "now", using the largest whole unit.
pub fn relative(timestamp: Timestamp, now: Timestamp) -> String {
    const UNITS: [(i64, &str); 6] = [
        (365 * 86_400, "year"),
        (30 * 86_400, "month"),
        (86_400, "day"),
        (3_600, "hour"),
        (60, "minute"),
        (1, "second"),
    ];
    let seconds = timestamp.as_second() - now.as_second();
    let distance = seconds.abs();
    if distance == 0 {
        return "now".into();
    }
    let (size, name) = UNITS
        .into_iter()
        .find(|(size, _)| distance >= *size)
        .unwrap_or(UNITS[5]);
    let count = distance / size;
    let unit = if count == 1 {
        name.to_owned()
    } else {
        format!("{name}s")
    };
    if seconds > 0 {
        format!("in {count} {unit}")
    } else {
        format!("{count} {unit} ago")
    }
}

/// Every IANA time zone name known to the system, sorted. Read once, then cached.
pub fn zone_names() -> &'static [String] {
    static NAMES: std::sync::OnceLock<Vec<String>> = std::sync::OnceLock::new();
    NAMES.get_or_init(|| {
        let mut names: Vec<String> = jiff::tz::db()
            .available()
            .map(|name| name.to_string())
            .collect();
        names.sort();
        names
    })
}

/// The system time zone's IANA name, if it has one.
pub fn system_zone_name() -> Option<String> {
    TimeZone::system().iana_name().map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXAMPLE: i64 = 1_700_000_000; // 2023-11-14T22:13:20Z

    fn example() -> Timestamp {
        Timestamp::from_second(EXAMPLE).unwrap()
    }

    fn parse_utc(input: &str) -> Result<(Timestamp, Source), Error> {
        parse(input, None, &TimeZone::UTC)
    }

    #[test]
    fn detects_unix_units() {
        assert_eq!(
            parse_utc("1700000000").unwrap(),
            (example(), Source::Unix(Unit::Seconds))
        );
        assert_eq!(
            parse_utc("1_700_000_000_000").unwrap(),
            (example(), Source::Unix(Unit::Milliseconds))
        );
        assert_eq!(
            parse_utc("1700000000000000").unwrap(),
            (example(), Source::Unix(Unit::Microseconds))
        );
        assert_eq!(
            parse_utc("1700000000000000000").unwrap(),
            (example(), Source::Unix(Unit::Nanoseconds))
        );
        assert_eq!(
            parse_utc("-86400").unwrap().0,
            Timestamp::from_second(-86_400).unwrap()
        );
        let (forced, _) = parse("1700000000", Some(Unit::Milliseconds), &TimeZone::UTC).unwrap();
        assert_eq!(forced, Timestamp::from_millisecond(1_700_000_000).unwrap());
    }

    #[test]
    fn parses_fractional_seconds() {
        let (timestamp, _) = parse_utc("1700000000.25").unwrap();
        assert_eq!(timestamp.as_millisecond(), 1_700_000_000_250);
        let (timestamp, _) = parse_utc("-0.5").unwrap();
        assert_eq!(timestamp.as_millisecond(), -500);
    }

    #[test]
    fn parses_date_formats() {
        assert_eq!(
            parse_utc("2023-11-14T22:13:20Z").unwrap(),
            (example(), Source::Rfc3339)
        );
        assert_eq!(
            parse_utc("2023-11-14T17:13:20-05:00").unwrap(),
            (example(), Source::Rfc3339)
        );
        assert_eq!(
            parse_utc("Tue, 14 Nov 2023 22:13:20 +0000").unwrap(),
            (example(), Source::Rfc2822)
        );
        assert_eq!(
            parse_utc("2023-11-14T17:13:20[America/New_York]").unwrap(),
            (example(), Source::ZonedIso)
        );
        let new_york = TimeZone::get("America/New_York").unwrap();
        assert_eq!(
            parse("2023-11-14 17:13:20", None, &new_york).unwrap(),
            (example(), Source::Civil)
        );
        let (midnight, _) = parse("2023-11-14", None, &TimeZone::UTC).unwrap();
        assert_eq!(midnight.as_second(), EXAMPLE - (22 * 3600 + 13 * 60 + 20));
        assert_eq!(parse_utc("  "), Err(Error::Empty));
        assert_eq!(parse_utc("next tuesday"), Err(Error::Unrecognized));
        assert_eq!(
            parse_utc("99999999999999999999999999"),
            Err(Error::OutOfRange)
        );
    }

    #[test]
    fn formats() {
        let new_york = TimeZone::get("America/New_York").unwrap();
        assert_eq!(
            format_in_zone(example(), &new_york),
            "2023-11-14 17:13:20 EST (-05:00)"
        );
        assert_eq!(
            rfc2822(example(), &TimeZone::UTC),
            "Tue, 14 Nov 2023 22:13:20 +0000"
        );
        assert_eq!(
            calendar_details(example(), &TimeZone::UTC),
            "Tuesday · week 46 · day 318 of the year"
        );
        assert_eq!(unix(example(), Unit::Milliseconds), 1_700_000_000_000);
        assert_eq!(
            unix(Timestamp::from_millisecond(-1).unwrap(), Unit::Seconds),
            -1
        );
    }

    #[test]
    fn relative_times() {
        let now = example();
        let at = |offset: i64| Timestamp::from_second(EXAMPLE + offset).unwrap();
        assert_eq!(relative(now, now), "now");
        assert_eq!(relative(at(1), now), "in 1 second");
        assert_eq!(relative(at(-90), now), "1 minute ago");
        assert_eq!(relative(at(3 * 3600 + 59), now), "in 3 hours");
        assert_eq!(relative(at(-2 * 86_400), now), "2 days ago");
        assert_eq!(relative(at(13 * 86_400), now), "in 13 days");
        assert_eq!(relative(at(-45 * 86_400), now), "1 month ago");
        assert_eq!(relative(at(400 * 86_400), now), "in 1 year");
    }

    #[test]
    fn knows_time_zones() {
        let names = zone_names();
        assert!(
            names.iter().any(|name| name == "Europe/London"),
            "{} zones",
            names.len()
        );
    }
}
