//! Plan states at the exact minute boundaries, and ordering.

use super::*;

fn slot(name: &str, minute: u32, answered: Option<&str>) -> PlanSlot {
    PlanSlot {
        medicine_id: 1,
        medicine_uid: "uid-1".into(),
        medicine_name: name.into(),
        minute,
        time: format!("{:02}:{:02}", minute / 60, minute % 60),
        dose: 2.0,
        unit: "tablets".into(),
        answered: answered.map(|s| (9, s.to_string())),
    }
}

#[test]
fn the_dose_is_due_from_its_minute_through_sixty_minutes_later() {
    let s = slot("A", 600, None);
    assert_eq!(state_of(&s, 599), SlotState::Upcoming);
    assert_eq!(state_of(&s, 600), SlotState::Due, "at 0 minutes late");
    assert_eq!(state_of(&s, 660), SlotState::Due, "at 60 minutes late");
    assert_eq!(state_of(&s, 661), SlotState::Overdue, "at 61 minutes late");
}

#[test]
fn the_window_constant_is_what_the_boundaries_rely_on() {
    assert_eq!(DUE_WINDOW_MINUTES, 60);
}

#[test]
fn midnight_and_end_of_day_slots_behave() {
    assert_eq!(state_of(&slot("A", 0, None), 0), SlotState::Due);
    assert_eq!(state_of(&slot("A", 0, None), 61), SlotState::Overdue);
    assert_eq!(state_of(&slot("A", 1439, None), 1438), SlotState::Upcoming);
    assert_eq!(state_of(&slot("A", 1439, None), 1439), SlotState::Due);
}

#[test]
fn any_status_other_than_taken_counts_as_skipped() {
    assert_eq!(
        state_of(&slot("A", 600, Some("skipped")), 0),
        SlotState::Skipped
    );
    assert_eq!(
        state_of(&slot("A", 600, Some("whatever")), 0),
        SlotState::Skipped
    );
}

#[test]
fn plan_carries_ids_minutes_until_and_the_slot_fields() {
    let p = plan(
        vec![slot("A", 600, Some("taken")), slot("B", 540, None)],
        570,
    );
    assert_eq!(
        (p[0].medicine_name.as_str(), p[0].minutes_until),
        ("B", -30)
    );
    assert_eq!(p[0].intake_id, None);
    assert_eq!((p[1].medicine_name.as_str(), p[1].minutes_until), ("A", 30));
    assert_eq!(p[1].intake_id, Some(9));
    assert_eq!(p[1].state, SlotState::Taken);
    assert_eq!((p[1].dose, p[1].unit.as_str()), (2.0, "tablets"));
}

#[test]
fn equal_times_sort_by_medicine_name() {
    let p = plan(
        vec![
            slot("Zinc", 480, None),
            slot("Aspirin", 480, None),
            slot("Mg", 480, None),
        ],
        0,
    );
    let names: Vec<_> = p.iter().map(|d| d.medicine_name.as_str()).collect();
    assert_eq!(names, ["Aspirin", "Mg", "Zinc"]);
}

#[test]
fn an_empty_day_plans_nothing() {
    assert!(plan(Vec::new(), 600).is_empty());
}
