//! Calendar arithmetic on plain `YYYY-MM-DD` strings.
//!
//! The browser owns the clock and the time zone: it sends its local date
//! and time with every request that depends on "now", so the server never
//! guesses a zone. That keeps the domain pure and testable, and avoids a
//! date-time dependency for a handful of day counts.

use chassis::Error;

/// A calendar date, validated.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: i32,
    pub month: u32,
    pub day: u32,
}

/// The years the app accepts. Wide enough for any real expiry or intake,
/// narrow enough that date arithmetic can never overflow.
pub const YEAR_RANGE: std::ops::RangeInclusive<i32> = 1900..=2200;

impl Date {
    /// Parse a strict `YYYY-MM-DD`; a time part may follow after a `T` or a
    /// space (`2026-10-30T08:00`) and is ignored here.
    pub fn parse(text: &str) -> Result<Self, Error> {
        let bad = || {
            Error::invalid(
                format!("'{}' is not a date", shown(text)),
                "use the form YYYY-MM-DD, for example 2026-10-30, with a year from 1900 to 2200",
            )
        };
        let bytes = text.as_bytes();
        let shaped = bytes.len() >= 10
            && bytes[..10].iter().enumerate().all(|(i, b)| match i {
                4 | 7 => *b == b'-',
                _ => b.is_ascii_digit(),
            })
            && (bytes.len() == 10 || matches!(bytes[10], b'T' | b' '));
        if !shaped {
            return Err(bad());
        }
        let num = |range: std::ops::Range<usize>| text[range].parse::<u32>().map_err(|_| bad());
        let (year, month, day) = (num(0..4)? as i32, num(5..7)?, num(8..10)?);
        if !YEAR_RANGE.contains(&year)
            || !(1..=12).contains(&month)
            || day == 0
            || day > days_in_month(year, month)
        {
            return Err(bad());
        }
        Ok(Self { year, month, day })
    }

    /// Days since 1970-01-01 (Howard Hinnant's civil-days algorithm).
    pub fn to_days(self) -> i64 {
        let y = i64::from(self.year) - i64::from(self.month <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = i64::from(self.month);
        let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    pub fn from_days(days: i64) -> Self {
        let z = days + 719_468;
        let era = z.div_euclid(146_097);
        let doe = z - era * 146_097;
        let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
        let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
        let mp = (5 * doy + 2) / 153;
        let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
        let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
        let year = (yoe + era * 400 + i64::from(month <= 2)) as i32;
        Self { year, month, day }
    }

    /// Whole days from `self` to `other` (negative when `other` is earlier).
    pub fn days_until(self, other: Date) -> i64 {
        other.to_days() - self.to_days()
    }

    pub fn plus_days(self, n: i64) -> Self {
        Self::from_days(self.to_days() + n)
    }

    /// 0 = Monday … 6 = Sunday.
    pub fn weekday(self) -> u32 {
        (self.to_days() + 3).rem_euclid(7) as u32
    }

    pub fn iso(self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        2 if year % 4 == 0 && (year % 100 != 0 || year % 400 == 0) => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Cut user text down for an error message (never echo a megabyte back).
fn shown(text: &str) -> String {
    text.chars().take(40).collect()
}

/// Minutes since midnight from `HH:MM`, or from the time part of
/// `YYYY-MM-DDTHH:MM` (seconds after it are ignored).
pub fn parse_minutes(text: &str) -> Result<u32, Error> {
    let bad = || {
        Error::invalid(
            format!("'{}' is not a time", shown(text)),
            "use the form HH:MM, for example 08:30",
        )
    };
    let t = match text.len() {
        5 => text,
        n if n > 10 => text.get(11..).ok_or_else(bad)?,
        _ => return Err(bad()),
    };
    let b = t.as_bytes();
    let shaped = b.len() >= 5
        && b[2] == b':'
        && [0, 1, 3, 4].iter().all(|i| b[*i].is_ascii_digit())
        && (b.len() == 5 || b[5] == b':');
    if !shaped {
        return Err(bad());
    }
    let (h, m): (u32, u32) = (
        t[..2].parse().map_err(|_| bad())?,
        t[3..5].parse().map_err(|_| bad())?,
    );
    if h > 23 || m > 59 {
        return Err(bad());
    }
    Ok(h * 60 + m)
}

/// `HH:MM` for a minute of the day (the canonical form stored for slots).
pub fn clock_text(minutes: u32) -> String {
    format!("{:02}:{:02}", minutes / 60, minutes % 60)
}

/// Canonical `YYYY-MM-DDTHH:MM` from what the browser sent, or an error.
pub fn parse_datetime(text: &str) -> Result<String, Error> {
    let date = Date::parse(text)?;
    if text.len() < 16 {
        return Err(Error::invalid(
            format!("'{}' has no time", shown(text)),
            "use the form YYYY-MM-DDTHH:MM, for example 2026-10-30T08:30",
        ));
    }
    Ok(format!(
        "{}T{}",
        date.iso(),
        clock_text(parse_minutes(text)?)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_through_days() {
        for text in ["1970-01-01", "2024-02-29", "2026-10-30", "2100-03-01"] {
            let d = Date::parse(text).unwrap();
            assert_eq!(Date::from_days(d.to_days()), d);
        }
    }

    #[test]
    fn weekday_is_monday_based() {
        // 2026-10-30 is a Friday.
        assert_eq!(Date::parse("2026-10-30").unwrap().weekday(), 4);
    }

    #[test]
    fn rejects_impossible_dates() {
        assert!(Date::parse("2026-02-30").is_err());
        assert!(Date::parse("soon").is_err());
    }

    #[test]
    fn rejects_sloppy_and_out_of_range_dates() {
        for bad in [
            "+202-10-30",
            "2026-1-30x",
            "2026-10-30garbage",
            "-200-01-01",
            "0000-01-01",
            "9999-12-31",
            "1899-12-31",
            "２０２６-10-30",
            "2026/10/30",
            "",
        ] {
            assert!(Date::parse(bad).is_err(), "{bad} must be refused");
        }
        assert!(Date::parse("2026-10-30 08:00").is_ok());
    }

    #[test]
    fn long_garbage_is_cut_in_the_error() {
        let err = Date::parse(&"x".repeat(5000)).unwrap_err();
        assert!(err.message.len() < 200);
    }

    #[test]
    fn times_are_strict() {
        for bad in [
            "8:00",
            "08:0",
            "08:30abc",
            "24:00",
            "08:60",
            "08-30",
            "2026-10-30",
            "2026-10-30T8:00",
            "",
        ] {
            assert!(parse_minutes(bad).is_err(), "{bad} must be refused");
        }
        assert_eq!(parse_minutes("2026-10-30T21:05:33").unwrap(), 1265);
    }

    #[test]
    fn datetimes_are_canonicalised() {
        assert_eq!(
            parse_datetime("2026-10-30 08:05").unwrap(),
            "2026-10-30T08:05"
        );
        assert_eq!(
            parse_datetime("2026-10-30T08:05:59").unwrap(),
            "2026-10-30T08:05"
        );
        assert!(parse_datetime("2026-10-30").is_err());
    }

    #[test]
    fn counts_days_across_a_leap_day() {
        let a = Date::parse("2024-02-28").unwrap();
        let b = Date::parse("2024-03-01").unwrap();
        assert_eq!(a.days_until(b), 2);
    }

    #[test]
    fn parses_times_with_and_without_a_date() {
        assert_eq!(parse_minutes("08:30").unwrap(), 510);
        assert_eq!(parse_minutes("2026-10-30T21:05").unwrap(), 1265);
        assert!(parse_minutes("25:00").is_err());
    }
}
