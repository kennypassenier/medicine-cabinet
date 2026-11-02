//! Horizon edges, run-out prediction and event content.

use super::*;
use crate::testkit::medicine;

fn today() -> Date {
    Date::parse("2026-10-30").unwrap()
}

fn ids(m: &Medicine) -> Vec<String> {
    wanted_events(std::slice::from_ref(m), today())
        .into_iter()
        .map(|e| e.external_id)
        .collect()
}

fn expiring(offset: i64) -> Medicine {
    let mut m = medicine(1, "A");
    m.stock = 0.0;
    m.effective_expiry = Some(today().plus_days(offset).iso());
    m
}

#[test]
fn expiry_today_and_exactly_at_the_horizon_are_planned() {
    assert_eq!(ids(&expiring(0)), ["expiry:1"]);
    assert_eq!(ids(&expiring(HORIZON_DAYS)), ["expiry:1"]);
}

#[test]
fn expiry_yesterday_and_beyond_the_horizon_are_not() {
    assert!(ids(&expiring(-1)).is_empty());
    assert!(ids(&expiring(HORIZON_DAYS + 1)).is_empty());
}

#[test]
fn course_end_follows_the_same_horizon() {
    let mut m = medicine(2, "Amox");
    m.stock = 0.0;
    m.course_ends_on = Some(today().plus_days(HORIZON_DAYS).iso());
    assert_eq!(ids(&m), ["course:2"]);
    m.course_ends_on = Some(today().plus_days(HORIZON_DAYS + 1).iso());
    assert!(ids(&m).is_empty());
    m.course_ends_on = Some(today().plus_days(-1).iso());
    assert!(ids(&m).is_empty());
}

#[test]
fn an_unparseable_stored_date_plans_nothing_instead_of_failing() {
    let mut m = expiring(5);
    m.effective_expiry = Some("garbage".into());
    m.course_ends_on = Some("garbage".into());
    assert!(ids(&m).is_empty());
}

#[test]
fn a_partial_weekday_schedule_stretches_the_run_out() {
    // Mondays and Thursdays, one tablet each: 2 a week.
    let mut m = medicine(1, "A");
    m.slots[0].days = 0b001_0001;
    m.stock = 14.0;
    // 14 / (2/7) = 49 days.
    assert_eq!(run_out_date(&m, today()).unwrap(), today().plus_days(49));
}

#[test]
fn a_larger_dose_shortens_the_run_out() {
    let mut m = medicine(1, "A");
    m.slots[0].dose = 3.0;
    m.stock = 30.0;
    assert_eq!(run_out_date(&m, today()).unwrap(), today().plus_days(10));
}

#[test]
fn several_slots_add_up() {
    let mut m = medicine(1, "A");
    let mut evening = m.slots[0].clone();
    evening.time = "20:00".into();
    evening.dose = 2.0;
    m.slots.push(evening);
    m.stock = 30.0;
    // 3 a day.
    assert_eq!(run_out_date(&m, today()).unwrap(), today().plus_days(10));
}

#[test]
fn the_run_out_rounds_down_to_whole_days() {
    let mut m = medicine(1, "A");
    m.stock = 2.5;
    m.slots[0].dose = 1.0;
    assert_eq!(run_out_date(&m, today()).unwrap(), today().plus_days(2));
}

#[test]
fn no_slots_inactive_or_as_needed_never_run_out() {
    let mut none = medicine(1, "A");
    none.slots.clear();
    assert!(run_out_date(&none, today()).is_none());
    let mut off = medicine(1, "A");
    off.active = false;
    assert!(run_out_date(&off, today()).is_none());
    let mut prn = medicine(1, "A");
    prn.as_needed = true;
    assert!(run_out_date(&prn, today()).is_none());
}

#[test]
fn reorder_event_is_never_in_the_past() {
    let mut m = medicine(1, "A");
    m.stock = 3.0; // runs out in 3 days, lead is 7
    let events = wanted_events(&[m], today());
    let reorder = events
        .iter()
        .find(|e| e.external_id == "reorder:1")
        .unwrap();
    assert_eq!(reorder.start, today().iso());
}

#[test]
fn a_run_out_beyond_the_horizon_plans_no_reorder() {
    let mut m = medicine(1, "A");
    m.stock = (HORIZON_DAYS + 1) as f64;
    assert!(ids(&m).is_empty());
    m.stock = HORIZON_DAYS as f64;
    assert_eq!(ids(&m), ["reorder:1"]);
}

#[test]
fn a_medicine_can_have_all_three_events_and_they_stay_unique() {
    let mut m = medicine(4, "A");
    m.effective_expiry = Some(today().plus_days(40).iso());
    m.course_ends_on = Some(today().plus_days(10).iso());
    let got = ids(&m);
    assert_eq!(got, ["expiry:4", "course:4", "reorder:4"]);
}

#[test]
fn events_of_several_medicines_and_inactive_ones_filtered() {
    let a = expiring(5);
    let mut b = expiring(6);
    b.id = 2;
    let mut off = expiring(7);
    off.id = 3;
    off.active = false;
    let got: Vec<_> = wanted_events(&[a, b, off], today())
        .into_iter()
        .map(|e| e.external_id)
        .collect();
    assert_eq!(got, ["expiry:1", "expiry:2"]);
}

#[test]
fn event_texts_describe_the_dates() {
    let mut m = medicine(1, "Eye drops");
    m.effective_expiry = Some(today().plus_days(5).iso());
    m.opened_on = Some("2026-10-01".into());
    m.shelf_life_days = Some(35);
    m.course_ends_on = Some(today().plus_days(2).iso());
    m.purpose = "Infection".into();
    let events = wanted_events(&[m], today());
    let by = |p: &str| {
        events
            .iter()
            .find(|e| e.external_id.starts_with(p))
            .unwrap()
    };
    assert_eq!(by("expiry").title, "Eye drops expires");
    assert_eq!(
        by("expiry").description,
        "Opened on 2026-10-01; good for 35 days after opening."
    );
    assert_eq!(by("course").title, "Last day of Eye drops");
    assert_eq!(by("course").description, "Infection");
    assert!(
        by("reorder")
            .description
            .starts_with("Eye drops runs out around ")
    );
}

#[test]
fn a_printed_expiry_has_its_own_note() {
    assert_eq!(
        wanted_events(&[expiring(5)], today())[0].description,
        "Printed expiry date."
    );
}

#[test]
fn the_fingerprint_changes_with_title_start_or_description_only() {
    let base = Wanted {
        external_id: "x".into(),
        title: "T".into(),
        start: "2026-11-01".into(),
        all_day: true,
        description: "D".into(),
    };
    let mut other_id = base.clone();
    other_id.external_id = "y".into();
    assert_eq!(base.fingerprint(), other_id.fingerprint());
    for change in 0..3 {
        let mut w = base.clone();
        match change {
            0 => w.title = "T2".into(),
            1 => w.start = "2026-11-02".into(),
            _ => w.description = "D2".into(),
        }
        assert_ne!(base.fingerprint(), w.fingerprint());
    }
}
