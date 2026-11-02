//! The server's own wall clock in the configured zone, as the pure inputs
//! the reminder and calendar loops need. Kept apart from the loops so the
//! daylight-saving cases can be tested with fixed instants.

use chrono::{DateTime, Datelike, Timelike, Utc};
use chrono_tz::Tz;

use crate::domain::date::Date;

/// A local day and minute of that day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocalNow {
    pub date: Date,
    /// Minutes since local midnight, as the wall clock shows them.
    pub minute: u32,
    pub hour: u32,
}

/// What the wall clock in `zone` shows at the instant `now`.
///
/// On a spring-forward day the minute jumps over the missing hour (a 02:30
/// slot is "late" from 03:00, so it is still reminded). On a fall-back day
/// the hour repeats and the minute runs backwards once; the sent-log, not
/// this value, is what keeps a reminder from going out twice.
pub fn local_now(now: DateTime<Utc>, zone: Tz) -> LocalNow {
    let local = now.with_timezone(&zone);
    LocalNow {
        date: Date {
            year: local.year(),
            month: local.month(),
            day: local.day(),
        },
        minute: local.hour() * 60 + local.minute(),
        hour: local.hour(),
    }
}

/// The current instant in the configured zone.
pub fn now_in(zone: Tz) -> LocalNow {
    local_now(Utc::now(), zone)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::plan::{PlanSlot, SlotState, plan};
    use chrono::TimeZone;

    const BRUSSELS: Tz = Tz::Europe__Brussels;

    fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap()
    }

    fn slot(minute: u32) -> PlanSlot {
        PlanSlot {
            medicine_id: 1,
            medicine_name: "X".into(),
            minute,
            time: format!("{:02}:{:02}", minute / 60, minute % 60),
            dose: 1.0,
            unit: String::new(),
            answered: None,
        }
    }

    #[test]
    fn spring_forward_skips_a_wall_clock_hour() {
        // 2026-03-29: 02:00 CET becomes 03:00 CEST (01:00 UTC).
        let before = local_now(at(2026, 3, 29, 0, 59), BRUSSELS);
        let after = local_now(at(2026, 3, 29, 1, 0), BRUSSELS);
        assert_eq!((before.hour, before.minute), (1, 119));
        assert_eq!((after.hour, after.minute), (3, 180));
    }

    #[test]
    fn a_slot_in_the_missing_hour_is_still_due_after_the_jump() {
        let now = local_now(at(2026, 3, 29, 1, 0), BRUSSELS);
        let doses = plan(vec![slot(150)], now.minute); // 02:30 does not exist
        assert_eq!(doses[0].state, SlotState::Due);
    }

    #[test]
    fn fall_back_repeats_an_hour_and_the_minute_steps_back_once() {
        // 2026-10-25: 03:00 CEST becomes 02:00 CET (01:00 UTC).
        let first = local_now(at(2026, 10, 25, 0, 30), BRUSSELS);
        let second = local_now(at(2026, 10, 25, 1, 30), BRUSSELS);
        assert_eq!((first.hour, first.minute), (2, 150));
        assert_eq!((second.hour, second.minute), (2, 150));
        assert_eq!(first.date, second.date);
    }

    #[test]
    fn the_local_date_rolls_over_at_local_midnight_not_utc() {
        let late = local_now(at(2026, 6, 30, 21, 59), BRUSSELS); // 23:59 CEST
        let next = local_now(at(2026, 6, 30, 22, 0), BRUSSELS); // 00:00 CEST
        assert_eq!(late.date.iso(), "2026-06-30");
        assert_eq!((next.date.iso().as_str(), next.minute), ("2026-07-01", 0));
    }

    #[test]
    fn the_digest_hour_is_a_local_hour() {
        // 08:00 UTC is 09:00 in Brussels in winter and 10:00 in summer.
        assert_eq!(local_now(at(2026, 1, 15, 8, 0), BRUSSELS).hour, 9);
        assert_eq!(local_now(at(2026, 7, 15, 8, 0), BRUSSELS).hour, 10);
    }
}
