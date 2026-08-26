//! Turning the date strings sources actually send into real timestamps.
//!
//! A deadline is the whole point of an opening — it drives the T-7/T-2/T-1
//! reminders and the "in 9 days" line on the card. Sources are inconsistent
//! about how they express one, so the parsing lives here rather than being
//! duplicated across adapters.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};

/// Parses Devpost's `submission_period_dates` and returns the closing date.
///
/// Devpost sends a human-readable range, in one of two shapes:
///
/// ```text
/// "Jun 24 - Aug 25, 2026"   both months named
/// "Aug 21 - 25, 2026"       second month implied by the first
/// ```
///
/// Only the end of the range matters. The implied-month case is the reason
/// this cannot be a one-line `parse_from_str`: the closing half is bare
/// `"25, 2026"` and has to borrow its month from the opening half.
///
/// The result is set to 23:59 UTC because Devpost gives a date and no time —
/// treating it as midnight would report a hackathon as closed a day early.
pub fn parse_devpost_range(range: &str) -> Option<DateTime<Utc>> {
    let (start, end) = range.split_once(" - ")?;
    let end = end.trim();

    // "Aug 25, 2026" — the month is present.
    let date = NaiveDate::parse_from_str(end, "%b %d, %Y")
        .or_else(|_| NaiveDate::parse_from_str(end, "%B %d, %Y"))
        .ok()
        .or_else(|| {
            // "25, 2026" — borrow the month from the start of the range.
            let month = start.trim().split_whitespace().next()?;
            let joined = format!("{month} {end}");
            NaiveDate::parse_from_str(&joined, "%b %d, %Y")
                .or_else(|_| NaiveDate::parse_from_str(&joined, "%B %d, %Y"))
                .ok()
        })?;

    Utc.from_local_datetime(&date.and_hms_opt(23, 59, 0)?).single()
}

/// Parses an ISO 8601 timestamp, as sent by Devfolio and Himalayas.
pub fn parse_iso(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Datelike;

    #[test]
    fn parses_a_range_naming_both_months() {
        let d = parse_devpost_range("Jun 24 - Aug 25, 2026").expect("should parse");
        assert_eq!((d.year(), d.month(), d.day()), (2026, 8, 25));
    }

    #[test]
    fn parses_a_range_with_an_implied_month() {
        // The shape a naive parser silently fails on, leaving the deadline
        // empty and the reminder never firing.
        let d = parse_devpost_range("Aug 21 - 25, 2026").expect("should parse");
        assert_eq!((d.year(), d.month(), d.day()), (2026, 8, 25));
    }

    #[test]
    fn parses_a_range_crossing_into_a_new_month() {
        let d = parse_devpost_range("Aug 10 - 26, 2026").expect("should parse");
        assert_eq!((d.year(), d.month(), d.day()), (2026, 8, 26));
    }

    #[test]
    fn the_deadline_lands_at_the_end_of_its_day() {
        // Midnight would mark a hackathon closed a full day early.
        let d = parse_devpost_range("Aug 21 - 25, 2026").unwrap();
        assert_eq!(d.time().to_string(), "23:59:00");
    }

    #[test]
    fn nonsense_returns_none_rather_than_a_wrong_date() {
        assert!(parse_devpost_range("").is_none());
        assert!(parse_devpost_range("coming soon").is_none());
        assert!(parse_devpost_range("Aug 21 25 2026").is_none());
        assert!(parse_devpost_range("Foo 21 - 25, 2026").is_none());
    }

    #[test]
    fn parses_devfolio_style_iso() {
        let d = parse_iso("2026-09-26T06:30:00+00:00").expect("should parse");
        assert_eq!((d.year(), d.month(), d.day()), (2026, 9, 26));
    }

    #[test]
    fn rejects_a_non_iso_string() {
        assert!(parse_iso("next Tuesday").is_none());
        assert!(parse_iso("2026-09-26").is_none());
    }
}
