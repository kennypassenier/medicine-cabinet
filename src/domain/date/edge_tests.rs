//! Date arithmetic edge cases: leap years, year boundaries, parse errors.

use super::*;

fn d(s: &str) -> Date {
    Date::parse(s).unwrap()
}

#[test]
fn leap_day_exists_only_in_real_leap_years() {
    assert!(Date::parse("2024-02-29").is_ok());
    assert!(Date::parse("2000-02-29").is_ok(), "divisible by 400");
    assert!(Date::parse("1900-02-29").is_err(), "century, not by 400");
    assert!(Date::parse("2100-02-29").is_err());
    assert!(Date::parse("2023-02-29").is_err());
}

#[test]
fn month_lengths_are_enforced() {
    for (text, ok) in [
        ("2026-04-31", false),
        ("2026-04-30", true),
        ("2026-06-31", false),
        ("2026-09-31", false),
        ("2026-11-31", false),
        ("2026-12-31", true),
        ("2026-01-31", true),
        ("2026-13-01", false),
        ("2026-00-10", false),
        ("2026-05-00", false),
    ] {
        assert_eq!(Date::parse(text).is_ok(), ok, "{text}");
    }
}

#[test]
fn plus_days_crosses_month_and_year_boundaries() {
    assert_eq!(d("2026-12-31").plus_days(1).iso(), "2027-01-01");
    assert_eq!(d("2027-01-01").plus_days(-1).iso(), "2026-12-31");
    assert_eq!(d("2026-01-31").plus_days(1).iso(), "2026-02-01");
    assert_eq!(d("2026-02-28").plus_days(1).iso(), "2026-03-01");
    assert_eq!(d("2024-02-28").plus_days(1).iso(), "2024-02-29");
    assert_eq!(d("2024-02-29").plus_days(1).iso(), "2024-03-01");
}

#[test]
fn plus_days_handles_long_spans_both_ways() {
    assert_eq!(d("2026-10-30").plus_days(0), d("2026-10-30"));
    assert_eq!(d("2026-10-30").plus_days(365).iso(), "2027-10-30");
    assert_eq!(d("2024-03-01").plus_days(365).iso(), "2025-03-01");
    assert_eq!(d("2024-02-29").plus_days(366).iso(), "2025-03-01");
    assert_eq!(d("2026-10-30").plus_days(-730).iso(), "2024-10-30");
    assert_eq!(d("1969-12-31").to_days(), -1);
    assert_eq!(d("1970-01-01").to_days(), 0);
}

#[test]
fn days_until_is_signed_and_antisymmetric() {
    let a = d("2026-03-01");
    let b = d("2027-03-01");
    assert_eq!(a.days_until(b), 365);
    assert_eq!(b.days_until(a), -365);
    assert_eq!(a.days_until(a), 0);
    assert_eq!(d("2023-12-31").days_until(d("2024-12-31")), 366);
}

#[test]
fn every_day_round_trips_for_four_years() {
    let start = d("2023-01-01").to_days();
    for n in 0..(4 * 366) {
        let date = Date::from_days(start + n);
        assert_eq!(date.to_days(), start + n);
        assert_eq!(Date::parse(&date.iso()).unwrap(), date);
    }
}

#[test]
fn weekday_cycles_monday_to_sunday() {
    // 2026-10-26 is a Monday.
    for (offset, expected) in (0..14).zip((0..7).cycle()) {
        assert_eq!(d("2026-10-26").plus_days(offset).weekday(), expected);
    }
    assert_eq!(d("1970-01-01").weekday(), 3, "a Thursday");
    assert_eq!(
        d("1969-12-31").weekday(),
        2,
        "a Wednesday, before the epoch"
    );
    assert_eq!(d("2024-02-29").weekday(), 3, "a Thursday");
}

#[test]
fn parse_ignores_a_trailing_time_part() {
    assert_eq!(d("2026-10-30T21:05"), d("2026-10-30"));
    assert_eq!(d("2026-10-30 21:05"), d("2026-10-30"));
}

#[test]
fn parse_rejects_malformed_text() {
    for bad in [
        "",
        "2026",
        "2026-10",
        "2026-1-5",
        "26-10-2026",
        "2026/10/30",
        "20261030",
        "abcd-ef-gh",
        "2026-10-3x",
        "2026-10-xx",
        "ééééééééééé",
    ] {
        assert!(Date::parse(bad).is_err(), "'{bad}' should not parse");
    }
}

#[test]
fn parse_errors_name_the_input_and_the_remedy() {
    let e = Date::parse("soon").unwrap_err();
    assert!(e.message.contains("soon"));
    assert!(!e.remedy.is_empty());
}

#[test]
fn iso_pads_to_four_two_two_digits() {
    let early = Date {
        year: 987,
        month: 3,
        day: 4,
    };
    assert_eq!(early.iso(), "0987-03-04");
}

#[test]
fn dates_order_chronologically() {
    assert!(d("2026-12-31") < d("2027-01-01"));
    assert!(d("2026-02-28") < d("2026-03-01"));
    assert_eq!(d("2026-05-05").min(d("2026-05-06")), d("2026-05-05"));
}

#[test]
fn minutes_accept_boundaries_and_reject_overflow() {
    assert_eq!(parse_minutes("00:00").unwrap(), 0);
    assert_eq!(parse_minutes("23:59").unwrap(), 1439);
    assert_eq!(parse_minutes("09:05").unwrap(), 545);
    assert_eq!(parse_minutes("2026-10-30 07:15").unwrap(), 435);
    for bad in [
        "24:00", "12:60", "9:05", "", "noon", "12", "12:", ":30", "-1:30", "ab:cd",
    ] {
        assert!(parse_minutes(bad).is_err(), "'{bad}' should not parse");
    }
}

#[test]
fn minutes_ignore_seconds_after_the_minute() {
    assert_eq!(parse_minutes("2026-10-30T21:05:59").unwrap(), 1265);
}
