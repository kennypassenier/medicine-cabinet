//! Overdue nudges that cross midnight.

use super::*;
use crate::domain::schedule::SlotInput;
use crate::testkit::medicine;

fn yesterday() -> Date {
    Date::parse("2026-10-26").unwrap()
}

fn late_medicine() -> Medicine {
    let mut m = medicine(1, "Melatonin");
    m.slots = vec![SlotInput {
        time: "23:30".into(),
        days: 127,
        dose: 1.0,
    }];
    m
}

fn logged(status: &str) -> Intake {
    Intake {
        id: 9,
        uid: "uid-9".into(),
        rev: 1,
        medicine_id: Some(1),
        medicine_name: String::new(),
        taken_at: "2026-10-27T00:10".into(),
        planned_for: Some("2026-10-26T23:30".into()),
        status: status.into(),
        dose: 1.0,
        note: String::new(),
    }
}

fn run(logged: &[Intake], minute: u32, sent: &HashSet<reminder_log::SentKey>) -> Vec<Reminder> {
    carryover_reminders(&[late_medicine()], logged, yesterday(), minute, sent)
}

#[test]
fn an_unlogged_late_dose_is_nudged_after_midnight() {
    let r = run(&[], 31, &HashSet::new()); // 00:31
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].kind, "dose.overdue");
    assert_eq!(r[0].key.1, "2026-10-26T23:30");
}

#[test]
fn a_logged_late_dose_stays_quiet() {
    assert!(run(&[logged("taken")], 31, &HashSet::new()).is_empty());
    assert!(run(&[logged("skipped")], 31, &HashSet::new()).is_empty());
}

#[test]
fn the_same_key_is_not_sent_twice() {
    let first = run(&[], 31, &HashSet::new());
    let sent: HashSet<_> = first.into_iter().map(|r| r.key).collect();
    assert!(run(&[], 45, &sent).is_empty());
}

#[test]
fn a_dose_more_than_180_minutes_late_gets_nothing() {
    assert!(run(&[], 180, &HashSet::new()).is_empty()); // 03:00 = 210 min late
}

#[test]
fn the_due_window_is_not_carried_over() {
    // 00:30 is exactly 60 minutes late: still "due", not yet overdue.
    assert!(run(&[], 30, &HashSet::new()).is_empty());
}
