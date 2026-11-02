//! Reminders with several medicines, mixed states, dedupe keys and the overdue limit.

use super::*;

fn dose(id: i64, name: &str, time: &str, state: SlotState, minutes_until: i64) -> PlannedDose {
    PlannedDose {
        medicine_id: id,
        medicine_name: name.into(),
        time: time.into(),
        dose: 1.0,
        unit: "tablets".into(),
        state,
        intake_id: None,
        minutes_until,
    }
}

fn attention(id: i64, name: &str, kind: &'static str, detail: &str) -> Attention {
    Attention {
        medicine_id: id,
        medicine_name: name.into(),
        kind,
        detail: detail.into(),
    }
}

#[test]
fn mixed_states_across_medicines_yield_only_due_and_recent_overdue() {
    let doses = [
        dose(1, "A", "08:00", SlotState::Taken, -30),
        dose(2, "B", "08:30", SlotState::Due, -10),
        dose(3, "C", "06:00", SlotState::Overdue, -150),
        dose(4, "D", "12:00", SlotState::Upcoming, 120),
        dose(5, "E", "01:00", SlotState::Overdue, -400),
    ];
    let r = dose_reminders("2026-10-30", &doses, &HashSet::new());
    let got: Vec<_> = r.iter().map(|r| (r.key.0, r.kind)).collect();
    assert_eq!(got, [(2, "dose.due"), (3, "dose.overdue")]);
}

#[test]
fn the_overdue_limit_is_inclusive_at_180_minutes() {
    let at = [dose(1, "A", "08:00", SlotState::Overdue, -180)];
    let past = [dose(1, "A", "08:00", SlotState::Overdue, -181)];
    assert_eq!(dose_reminders("d", &at, &HashSet::new()).len(), 1);
    assert!(dose_reminders("d", &past, &HashSet::new()).is_empty());
    assert_eq!(OVERDUE_LIMIT_MINUTES, 180);
}

#[test]
fn the_key_is_medicine_slot_and_kind() {
    let doses = [dose(7, "A", "08:00", SlotState::Due, 0)];
    let r = dose_reminders("2026-10-30", &doses, &HashSet::new());
    assert_eq!(
        r[0].key,
        (7, "2026-10-30T08:00".to_string(), "due".to_string())
    );
    let late = [dose(7, "A", "08:00", SlotState::Overdue, -90)];
    let r = dose_reminders("2026-10-30", &late, &HashSet::new());
    assert_eq!(r[0].key.2, "overdue");
}

#[test]
fn due_then_overdue_is_two_different_nudges() {
    let mut sent = HashSet::new();
    sent.insert((1, "2026-10-30T08:00".to_string(), "due".to_string()));
    let late = [dose(1, "A", "08:00", SlotState::Overdue, -70)];
    assert_eq!(dose_reminders("2026-10-30", &late, &sent).len(), 1);
}

#[test]
fn a_sent_key_of_another_day_or_medicine_does_not_suppress() {
    let mut sent = HashSet::new();
    sent.insert((1, "2026-10-29T08:00".to_string(), "due".to_string()));
    sent.insert((2, "2026-10-30T08:00".to_string(), "due".to_string()));
    let doses = [dose(1, "A", "08:00", SlotState::Due, -1)];
    assert_eq!(dose_reminders("2026-10-30", &doses, &sent).len(), 1);
}

#[test]
fn two_slots_of_one_medicine_are_reminded_separately() {
    let doses = [
        dose(1, "A", "08:00", SlotState::Due, -1),
        dose(1, "A", "20:00", SlotState::Due, -1),
    ];
    let r = dose_reminders("2026-10-30", &doses, &HashSet::new());
    assert_eq!(r.len(), 2);
    assert_ne!(r[0].key, r[1].key);
}

#[test]
fn only_the_unsent_one_of_several_comes_back() {
    let doses = [
        dose(1, "A", "08:00", SlotState::Due, -1),
        dose(2, "B", "08:00", SlotState::Due, -1),
    ];
    let sent: HashSet<SentKey> =
        HashSet::from([(1, "2026-10-30T08:00".to_string(), "due".to_string())]);
    let r = dose_reminders("2026-10-30", &doses, &sent);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].key.0, 2);
}

#[test]
fn texts_name_the_medicine_amount_and_time() {
    let due = dose_reminders(
        "d",
        &[dose(1, "Vitamin D", "08:00", SlotState::Due, -1)],
        &HashSet::new(),
    );
    assert_eq!(
        due[0].text,
        "Time for Vitamin D (1 tablet), planned at 08:00."
    );
    let late = dose_reminders(
        "d",
        &[dose(1, "Vitamin D", "08:00", SlotState::Overdue, -70)],
        &HashSet::new(),
    );
    assert_eq!(
        late[0].text,
        "Vitamin D was planned at 08:00 and is not logged yet."
    );
}

#[test]
fn no_doses_no_reminders() {
    assert!(dose_reminders("d", &[], &HashSet::new()).is_empty());
    assert!(attention_reminders("d", &[], &HashSet::new()).is_empty());
}

#[test]
fn the_digest_has_one_reminder_per_item_keyed_by_day_and_kind() {
    let items = [
        attention(1, "A", "low_stock", "Only 2 tablets left."),
        attention(1, "A", "expired", "Expired on 2026-10-01."),
        attention(2, "B", "low_stock", "Only 1 left."),
    ];
    let r = attention_reminders("2026-10-30", &items, &HashSet::new());
    assert_eq!(r.len(), 3);
    assert!(r.iter().all(|r| r.kind == "cabinet.attention"));
    assert_eq!(
        r[0].key,
        (1, "2026-10-30".to_string(), "low_stock".to_string())
    );
    assert_eq!(r[0].text, "A: Only 2 tablets left.");
    let keys: HashSet<_> = r.iter().map(|r| r.key.clone()).collect();
    assert_eq!(keys.len(), 3);
}

#[test]
fn the_digest_resends_on_a_new_day_but_not_the_same_day() {
    let items = [attention(1, "A", "low_stock", "Low.")];
    let sent: HashSet<SentKey> =
        HashSet::from([(1, "2026-10-29".to_string(), "low_stock".to_string())]);
    assert_eq!(attention_reminders("2026-10-30", &items, &sent).len(), 1);
    assert!(attention_reminders("2026-10-29", &items, &sent).is_empty());
}

#[test]
fn a_sent_kind_does_not_hide_a_different_kind_of_the_same_medicine() {
    let items = [
        attention(1, "A", "low_stock", "Low."),
        attention(1, "A", "expiring_soon", "Good until 2026-11-10."),
    ];
    let sent: HashSet<SentKey> =
        HashSet::from([(1, "2026-10-30".to_string(), "low_stock".to_string())]);
    let r = attention_reminders("2026-10-30", &items, &sent);
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].key.2, "expiring_soon");
}
