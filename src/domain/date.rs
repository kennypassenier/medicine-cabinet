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

impl Date {
    /// Parse `YYYY-MM-DD` (a trailing time part is ignored).
    pub fn parse(text: &str) -> Result<Self, Error> {
        let bad = || {
            Error::invalid(
                format!("'{text}' is not a date"),
                "use the form YYYY-MM-DD, for example 2026-10-30",
            )
        };
        let head = text.get(..10).ok_or_else(bad)?;
        let mut parts = head.split('-');
        let (y, m, d) = (parts.next(), parts.next(), parts.next());
        let year: i32 = y.and_then(|v| v.parse().ok()).ok_or_else(bad)?;
        let month: u32 = m.and_then(|v| v.parse().ok()).ok_or_else(bad)?;
        let day: u32 = d.and_then(|v| v.parse().ok()).ok_or_else(bad)?;
        if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
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

/// Minutes since midnight from `HH:MM` (the time part of `YYYY-MM-DDTHH:MM`
/// is accepted too).
pub fn parse_minutes(text: &str) -> Result<u32, Error> {
    let t = text.rsplit(['T', ' ']).next().unwrap_or(text);
    let bad = || Error::invalid(format!("'{text}' is not a time"), "use the form HH:MM");
    let (h, m) = t.split_once(':').ok_or_else(bad)?;
    let h: u32 = h.parse().map_err(|_| bad())?;
    let m: u32 = m.get(..2).unwrap_or(m).parse().map_err(|_| bad())?;
    if h > 23 || m > 59 {
        return Err(bad());
    }
    Ok(h * 60 + m)
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

#[cfg(test)]
mod edge_tests;
