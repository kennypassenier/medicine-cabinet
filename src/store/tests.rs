//! The stores against a real (in-memory) sqlite database with the app's own migrations.

use chassis::storage::rusqlite::Connection;

use super::{categories, forms, intakes, medicines, symptoms};
use crate::domain::date::Date;
use crate::domain::medicine::MedicineInput;
use crate::store::intakes::{IntakeEdit, IntakeInput};

pub(super) fn db() -> Connection {
    crate::testkit::sync_db()
}

pub(super) fn today() -> Date {
    Date::parse("2026-11-01").unwrap()
}

pub(super) fn medicine(name: &str, stock: f64) -> MedicineInput {
    MedicineInput {
        name: name.into(),
        purpose: String::new(),
        form_id: None,
        strength: String::new(),
        notes: String::new(),
        active: true,
        as_needed: false,
        min_gap_minutes: None,
        max_per_day: None,
        stock,
        stock_unit: "tablets".into(),
        low_stock_at: None,
        expires_on: None,
        opened_on: None,
        shelf_life_days: None,
        course_ends_on: None,
        symptoms: vec!["headache".into()],
    }
}

pub(super) fn taken(medicine_id: i64, at: &str, dose: f64) -> IntakeInput {
    IntakeInput {
        medicine_id,
        taken_at: at.into(),
        planned_for: None,
        status: "taken".into(),
        dose: Some(dose),
        note: String::new(),
    }
}

#[test]
fn a_deleted_medicine_hides_and_comes_back_with_its_history() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("Ibuprofen", 10.0)).unwrap();
    intakes::record(&tx, &taken(id, "2026-11-01T08:00", 1.0)).unwrap();
    medicines::delete(&tx, id).unwrap();
    assert!(medicines::get(&tx, id, today()).is_err());
    assert!(medicines::list(&tx, today()).unwrap().is_empty());
    assert!(intakes::list(&tx, "", 50).unwrap().is_empty());
    medicines::restore(&tx, id).unwrap();
    let back = medicines::get(&tx, id, today()).unwrap();
    assert_eq!(back.stock, 9.0);
    assert_eq!(intakes::list(&tx, "", 50).unwrap().len(), 1);
    assert!(
        medicines::restore(&tx, id).is_err(),
        "only a deleted medicine restores"
    );
}

#[test]
fn purge_only_removes_what_is_past_the_window() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let fresh = medicines::insert(&tx, &medicine("Fresh", 1.0)).unwrap();
    let old = medicines::insert(&tx, &medicine("Old", 1.0)).unwrap();
    medicines::delete(&tx, fresh).unwrap();
    medicines::delete(&tx, old).unwrap();
    tx.execute(
        "UPDATE medicines SET deleted_at = datetime('now', '-15 days') WHERE id = ?1",
        [old],
    )
    .unwrap();
    assert_eq!(medicines::purge_deleted(&tx).unwrap(), 1);
    medicines::restore(&tx, fresh).unwrap();
    assert!(medicines::restore(&tx, old).is_err());
}

#[test]
fn editing_a_dose_moves_the_stock_by_the_difference() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("Vitamin D", 10.0)).unwrap();
    let intake = intakes::record(&tx, &taken(id, "2026-11-01T09:30", 1.0)).unwrap();
    intakes::edit(
        &tx,
        intake,
        &IntakeEdit {
            taken_at: "2026-11-01T08:00".into(),
            dose: 3.0,
            note: "late".into(),
        },
    )
    .unwrap();
    assert_eq!(medicines::get(&tx, id, today()).unwrap().stock, 7.0);
    let row = &intakes::list(&tx, "", 5).unwrap()[0];
    assert_eq!(
        (row.taken_at.as_str(), row.dose, row.note.as_str()),
        ("2026-11-01T08:00", 3.0, "late")
    );
    assert!(
        intakes::edit(
            &tx,
            intake,
            &IntakeEdit {
                taken_at: "x".into(),
                dose: 0.0,
                note: String::new()
            }
        )
        .is_err()
    );
}

#[test]
fn removing_and_relogging_an_intake_round_trips_the_stock() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("Cetirizine", 5.0)).unwrap();
    let input = taken(id, "2026-11-01T10:00", 2.0);
    let first = intakes::record(&tx, &input).unwrap();
    intakes::undo(&tx, first).unwrap();
    assert_eq!(medicines::get(&tx, id, today()).unwrap().stock, 5.0);
    intakes::record(&tx, &input).unwrap();
    assert_eq!(medicines::get(&tx, id, today()).unwrap().stock, 3.0);
}

#[test]
fn deleting_a_symptom_reports_its_links_and_recreating_restores_them() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let a = medicines::insert(&tx, &medicine("A", 1.0)).unwrap();
    let b = medicines::insert(&tx, &medicine("B", 1.0)).unwrap();
    let head = categories::create(&tx, "Head", &[]).unwrap();
    let symptom = symptoms::list(&tx).unwrap()[0].id;
    symptoms::update(&tx, symptom, "headache", Some(head)).unwrap();
    let gone = symptoms::delete(&tx, symptom).unwrap();
    assert_eq!(gone.medicine_ids.len(), 2);
    assert!(medicines::get(&tx, a, today()).unwrap().symptoms.is_empty());
    symptoms::create(&tx, &gone.name, gone.category_id, &gone.medicine_ids).unwrap();
    for id in [a, b] {
        assert_eq!(
            medicines::get(&tx, id, today()).unwrap().symptoms,
            vec!["headache"]
        );
    }
    let again = &symptoms::list(&tx).unwrap()[0];
    assert_eq!(again.category.as_deref(), Some("Head"));
}

#[test]
fn deleting_a_category_reports_its_symptoms_and_recreating_refiles_them() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    medicines::insert(&tx, &medicine("A", 1.0)).unwrap();
    let head = categories::create(&tx, "Head", &[]).unwrap();
    let symptom = symptoms::list(&tx).unwrap()[0].id;
    symptoms::update(&tx, symptom, "headache", Some(head)).unwrap();
    let gone = categories::delete(&tx, head).unwrap();
    assert_eq!(gone.symptom_ids, vec![symptom]);
    assert_eq!(symptoms::list(&tx).unwrap()[0].category, None);
    categories::create(&tx, &gone.name, &gone.symptom_ids).unwrap();
    assert_eq!(
        symptoms::list(&tx).unwrap()[0].category.as_deref(),
        Some("Head")
    );
    assert!(
        categories::create(&tx, "head", &[]).is_err(),
        "names are unique without regard to case"
    );
}

#[test]
fn deleting_a_form_reports_its_medicines_and_recreating_sets_it_again() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let tablet = forms::list(&tx)
        .unwrap()
        .into_iter()
        .find(|f| f.name == "Tablet")
        .unwrap();
    let mut input = medicine("A", 1.0);
    input.form_id = Some(tablet.id);
    let id = medicines::insert(&tx, &input).unwrap();
    let gone = forms::delete(&tx, tablet.id).unwrap();
    assert_eq!(gone.medicine_ids, vec![id]);
    assert_eq!(medicines::get(&tx, id, today()).unwrap().form, "");
    forms::create(&tx, &gone.name, &gone.unit, &gone.medicine_ids).unwrap();
    assert_eq!(medicines::get(&tx, id, today()).unwrap().form, "Tablet");
}

mod history_stock;
mod intakes_more;
mod lists_more;
mod logs_more;
mod medicines_more;
mod sync_tests;
