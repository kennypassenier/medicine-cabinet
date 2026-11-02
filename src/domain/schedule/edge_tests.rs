//! Slot validation and the as-needed dosing guard at their boundaries.

use super::*;

fn slot(time: &str, days: u8, dose: f64) -> SlotInput {
    SlotInput {
        time: time.into(),
        days,
        dose,
    }
}

fn ago(minutes: &[i64]) -> Vec<PastDose> {
    minutes
        .iter()
        .map(|&m| PastDose { minutes_ago: m })
        .collect()
}

#[test]
fn slot_json_defaults_to_every_day_and_one_dose() {
    let s: SlotInput = serde_json::from_str(r#"{"time":"08:00"}"#).unwrap();
    assert_eq!((s.days, s.dose), (127, 1.0));
}

#[test]
fn slot_day_mask_boundaries() {
    assert!(slot("08:00", 127, 1.0).validated().is_ok());
    assert!(
        slot("08:00", 128, 1.0).validated().is_err(),
        "bit 7 is no weekday"
    );
    assert!(slot("08:00", 255, 1.0).validated().is_err());
    assert!(slot("08:00", 0, 1.0).validated().is_err());
    assert!(slot("08:00", 64, 1.0).validated().is_ok(), "Sunday only");
}

#[test]
fn slot_time_and_dose_boundaries() {
    assert!(slot("00:00", 1, 0.25).validated().is_ok());
    assert!(slot("23:59", 1, 1.0).validated().is_ok());
    assert!(slot("24:00", 1, 1.0).validated().is_err());
    assert!(slot("", 1, 1.0).validated().is_err());
    assert!(slot("08:00", 1, -1.0).validated().is_err());
    assert!(slot("08:00", 1, 0.0).validated().is_err());
}

#[test]
fn a_slot_runs_only_on_its_weekdays_across_a_week() {
    let weekend = slot("09:00", 0b110_0000, 1.0);
    let monday = Date::parse("2026-10-26").unwrap();
    let runs: Vec<bool> = (0..7)
        .map(|n| weekend.runs_on(monday.plus_days(n)))
        .collect();
    assert_eq!(runs, [false, false, false, false, false, true, true]);
}

#[test]
fn the_gap_is_open_exactly_when_it_has_elapsed() {
    let gap = Some(240);
    assert!(!check_dose(&ago(&[239]), gap, None).allowed);
    let at = check_dose(&ago(&[240]), gap, None);
    assert!(at.allowed, "exactly the gap later is fine");
    assert_eq!(at.wait_minutes, None);
    assert!(check_dose(&ago(&[241]), gap, None).allowed);
}

#[test]
fn a_one_minute_wait_is_reported_as_one() {
    let v = check_dose(&ago(&[239]), Some(240), None);
    assert_eq!(v.wait_minutes, Some(1));
    assert!(v.reason.unwrap().contains("wait 1 more"));
}

#[test]
fn just_taken_waits_the_full_gap() {
    let v = check_dose(&ago(&[0]), Some(120), None);
    assert_eq!(v.wait_minutes, Some(120));
}

#[test]
fn only_the_most_recent_dose_counts_for_the_gap() {
    let v = check_dose(&ago(&[600, 30, 400]), Some(60), None);
    assert_eq!(v.wait_minutes, Some(30));
}

#[test]
fn no_limits_means_always_allowed() {
    assert!(check_dose(&ago(&[0, 1, 2, 3]), None, None).allowed);
}

#[test]
fn the_daily_maximum_window_is_exclusive_at_24_hours() {
    // A dose exactly 24h ago is outside the window.
    assert!(check_dose(&ago(&[1440, 100]), None, Some(2)).allowed);
    // One minute inside the window still counts.
    assert!(!check_dose(&ago(&[1439, 100]), None, Some(2)).allowed);
}

#[test]
fn the_daily_maximum_reports_when_the_oldest_dose_drops_out() {
    let v = check_dose(&ago(&[60, 600]), None, Some(2));
    assert!(!v.allowed);
    assert_eq!(v.wait_minutes, Some(1440 - 600));
    assert!(v.reason.unwrap().contains("maximum (2)"));
}

#[test]
fn a_maximum_of_zero_blocks_everything() {
    let v = check_dose(&[], None, Some(0));
    assert!(!v.allowed);
    assert_eq!(v.wait_minutes, None, "no dose to wait for");
}

#[test]
fn the_gap_is_checked_before_the_daily_maximum() {
    let v = check_dose(&ago(&[10, 20, 30]), Some(60), Some(3));
    assert_eq!(v.wait_minutes, Some(50));
    assert!(v.reason.unwrap().contains("last dose"));
}

#[test]
fn old_doses_outside_the_day_do_not_trigger_the_maximum() {
    let past = ago(&[2000, 3000, 4000]);
    assert!(check_dose(&past, None, Some(1)).allowed);
}
