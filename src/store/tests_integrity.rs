//! Stock, soft-delete and validation guarantees of the stores.

use chassis::Kind;

use super::sync_log::Coll;
use super::tests::{db, ident, medicine, taken, today};
use super::{forms, intakes, medicines, slots, stock, symptoms};
use crate::domain::schedule::SlotInput;
use crate::store::intakes::{IntakeEdit, IntakeInput};

fn stock_of(tx: &chassis::storage::rusqlite::Transaction, id: i64) -> f64 {
    medicines::get(tx, id, today()).unwrap().stock
}

#[test]
fn undo_gives_back_only_what_the_stock_could_give() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("Vitamin D", 1.0)).unwrap();
    let intake = intakes::record(&tx, &taken(id, "2026-11-01T08:00", 2.0)).unwrap();
    assert_eq!(stock_of(&tx, id), 0.0);
    intakes::undo(&tx, intake).unwrap();
    assert_eq!(stock_of(&tx, id), 1.0, "not 2: only 1 ever left the box");
}

#[test]
fn editing_a_dose_after_a_clamp_keeps_the_stock_consistent() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 1.0)).unwrap();
    let intake = intakes::record(&tx, &taken(id, "2026-11-01T08:00", 3.0)).unwrap();
    let edit = |dose| IntakeEdit {
        taken_at: "2026-11-01T08:00".into(),
        dose,
        note: String::new(),
    };
    intakes::edit(&tx, intake, &edit(1.0)).unwrap();
    assert_eq!(stock_of(&tx, id), 0.0);
    intakes::edit(&tx, intake, &edit(0.5)).unwrap();
    assert_eq!(stock_of(&tx, id), 0.5);
    intakes::undo(&tx, intake).unwrap();
    assert_eq!(stock_of(&tx, id), 1.0);
}

#[test]
fn a_scheduled_dose_cannot_be_logged_twice() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 5.0)).unwrap();
    let mut input = taken(id, "2026-11-01T08:01", 1.0);
    input.planned_for = Some("2026-11-01T08:00".into());
    intakes::record(&tx, &input).unwrap();
    let again = intakes::record(&tx, &input).unwrap_err();
    assert_eq!(again.kind, Kind::Invalid);
    assert_eq!(stock_of(&tx, id), 4.0, "the second tap took nothing");
}

#[test]
fn nothing_can_touch_a_soft_deleted_medicine() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("Gone", 5.0)).unwrap();
    medicines::delete(&tx, id).unwrap();
    assert!(intakes::record(&tx, &taken(id, "2026-11-01T08:00", 1.0)).is_err());
    assert!(medicines::set_active(&tx, id, false).is_err());
    assert!(stock::adjust_stock(&tx, id, 3.0).is_err());
    assert!(medicines::update(&tx, id, &medicine("Renamed", 1.0), true).is_err());
    medicines::restore(&tx, id).unwrap();
    let back = medicines::get(&tx, id, today()).unwrap();
    assert_eq!(
        (back.name.as_str(), back.stock, back.active),
        ("Gone", 5.0, true)
    );
}

#[test]
fn bad_doses_and_times_are_refused_before_any_write() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 5.0)).unwrap();
    for dose in [0.0, -2.0, f64::NAN, f64::INFINITY, 1e300] {
        assert!(intakes::record(&tx, &taken(id, "2026-11-01T08:00", dose)).is_err());
    }
    for at in [
        "2026-11-01",
        "soon",
        "2026-11-01T25:00",
        "9999-01-01T08:00",
        "",
    ] {
        assert!(intakes::record(&tx, &taken(id, at, 1.0)).is_err(), "{at}");
    }
    let skipped = IntakeInput {
        status: "later".into(),
        ..taken(id, "2026-11-01T08:00", 1.0)
    };
    assert!(intakes::record(&tx, &skipped).is_err());
    assert_eq!(stock_of(&tx, id), 5.0);
    assert!(intakes::list(&tx, "", 10).unwrap().is_empty());
}

#[test]
fn intake_times_are_stored_in_one_canonical_shape() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 5.0)).unwrap();
    intakes::record(&tx, &taken(id, "2026-11-01 08:05:44", 1.0)).unwrap();
    assert_eq!(
        intakes::list(&tx, "", 10).unwrap()[0].taken_at,
        "2026-11-01T08:05"
    );
}

#[test]
fn day_filter_treats_percent_and_underscore_literally() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 5.0)).unwrap();
    intakes::record(&tx, &taken(id, "2026-11-01T08:00", 1.0)).unwrap();
    for pattern in ["%", "2026-11-0_", "____-__-__", "2026%"] {
        assert!(
            intakes::list(&tx, pattern, 10).unwrap().is_empty(),
            "{pattern}"
        );
    }
    assert_eq!(intakes::list(&tx, "2026-11-01", 10).unwrap().len(), 1);
}

#[test]
fn counts_leave_out_soft_deleted_medicines() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let tablet = forms::list(&tx)
        .unwrap()
        .into_iter()
        .find(|f| f.name == "Tablet")
        .unwrap();
    let mut input = medicine("A", 1.0);
    input.form_id = Some(ident(&tx, Coll::Forms, &tablet.uid));
    let id = medicines::insert(&tx, &input).unwrap();
    medicines::delete(&tx, id).unwrap();
    assert_eq!(symptoms::list(&tx).unwrap()[0].medicines, 0);
    let count = |tx: &chassis::storage::rusqlite::Transaction| {
        forms::list(tx)
            .unwrap()
            .into_iter()
            .find(|f| f.uid == tablet.uid)
            .unwrap()
            .medicines
    };
    assert_eq!(count(&tx), 0);
    medicines::restore(&tx, id).unwrap();
    assert_eq!(count(&tx), 1);
    assert_eq!(symptoms::list(&tx).unwrap()[0].medicines, 1);
}

#[test]
fn unknown_references_are_the_callers_error_not_the_databases() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let mut input = medicine("A", 1.0);
    input.form_id = Some(9999);
    assert_eq!(
        medicines::insert(&tx, &input).unwrap_err().kind,
        Kind::NotFound
    );
    let err = symptoms::create(&tx, "cough", Some(9999), &[]).unwrap_err();
    assert_eq!(err.kind, Kind::Invalid, "{err}");
    assert!(stock::adjust_stock(&tx, 4242, 1.0).is_err());
}

#[test]
fn hand_corrections_stay_within_bounds() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 2.0)).unwrap();
    stock::adjust_stock(&tx, id, -50.0).unwrap();
    assert_eq!(stock_of(&tx, id), 0.0);
    stock::adjust_stock(&tx, id, 1_000_000.0).unwrap();
    stock::adjust_stock(&tx, id, 1_000_000.0).unwrap();
    assert_eq!(stock_of(&tx, id), 1_000_000.0);
    for bad in [f64::NAN, f64::INFINITY, 1e300] {
        assert!(stock::adjust_stock(&tx, id, bad).is_err());
    }
}

#[test]
fn a_failed_create_leaves_no_half_written_medicine() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let mut input = medicine("Half", 1.0);
    input.form_id = Some(9999);
    assert!(medicines::insert(&tx, &input).is_err());
    assert!(medicines::list(&tx, today()).unwrap().is_empty());
    let id = medicines::insert(&tx, &medicine("Whole", 1.0)).unwrap();
    let slot = SlotInput {
        time: "08:00".into(),
        days: 127,
        dose: 1.0,
    };
    slots::replace(&tx, id, &[slot]).unwrap();
    assert_eq!(medicines::get(&tx, id, today()).unwrap().slots.len(), 1);
}
