//! Pairing today's slots with the intakes that answered them.

use super::*;
use crate::domain::schedule::SlotInput;
use crate::testkit::medicine;

fn today() -> Date {
    Date::parse("2026-10-26").unwrap() // a Monday
}

fn intake(id: i64, medicine_id: i64, planned: Option<&str>, status: &str) -> Intake {
    Intake {
        id,
        uid: format!("uid-{id}"),
        rev: 1,
        medicine_id: Some(medicine_id),
        medicine_name: String::new(),
        taken_at: "2026-10-26T08:05".into(),
        planned_for: planned.map(String::from),
        status: status.into(),
        dose: 1.0,
        note: String::new(),
    }
}

#[test]
fn a_daily_slot_becomes_one_unanswered_plan_slot() {
    let slots = day_slots(&[medicine(1, "A")], &[], today());
    assert_eq!(slots.len(), 1);
    assert_eq!((slots[0].minute, slots[0].time.as_str()), (480, "08:00"));
    assert!(slots[0].answered.is_none());
    assert_eq!(slots[0].unit, "capsules");
}

#[test]
fn a_matching_planned_for_answers_the_slot_with_its_status() {
    let logged = [intake(5, 1, Some("2026-10-26T08:00"), "skipped")];
    let slots = day_slots(&[medicine(1, "A")], &logged, today());
    assert_eq!(slots[0].answered, Some((5, "skipped".to_string())));
}

#[test]
fn an_intake_for_another_medicine_day_or_slot_does_not_answer() {
    let logged = [
        intake(1, 2, Some("2026-10-26T08:00"), "taken"),
        intake(2, 1, Some("2026-10-25T08:00"), "taken"),
        intake(3, 1, Some("2026-10-26T20:00"), "taken"),
        intake(4, 1, None, "taken"),
    ];
    let slots = day_slots(&[medicine(1, "A")], &logged, today());
    assert!(slots[0].answered.is_none());
}

#[test]
fn slots_only_appear_on_their_weekdays() {
    let mut m = medicine(1, "A");
    m.slots[0].days = 0b10; // Tuesday only
    assert!(day_slots(std::slice::from_ref(&m), &[], today()).is_empty());
    assert_eq!(day_slots(&[m], &[], today().plus_days(1)).len(), 1);
}

#[test]
fn inactive_as_needed_and_finished_courses_have_no_slots() {
    let mut off = medicine(1, "A");
    off.active = false;
    let mut prn = medicine(2, "B");
    prn.as_needed = true;
    let mut over = medicine(3, "C");
    over.course_over = true;
    assert!(day_slots(&[off, prn, over], &[], today()).is_empty());
}

#[test]
fn several_slots_and_medicines_all_appear_with_their_doses() {
    let mut a = medicine(1, "A");
    a.slots.push(SlotInput {
        time: "20:30".into(),
        days: 127,
        dose: 2.0,
    });
    let b = medicine(2, "B");
    let slots = day_slots(&[a, b], &[], today());
    assert_eq!(slots.len(), 3);
    assert!(
        slots
            .iter()
            .any(|s| s.time == "20:30" && s.dose == 2.0 && s.minute == 1230)
    );
}
