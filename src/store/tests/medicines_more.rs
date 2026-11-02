//! Medicine store: switching on/off, stock floor, ordering, symptom links.

use super::*;

#[test]
fn set_active_switches_a_medicine_and_reports_unknown_ids() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 5.0)).unwrap();
    medicines::set_active(&tx, id, false).unwrap();
    assert!(!medicines::get(&tx, id, today()).unwrap().active);
    medicines::set_active(&tx, id, true).unwrap();
    assert!(medicines::get(&tx, id, today()).unwrap().active);
    assert!(medicines::set_active(&tx, 999, true).is_err());
}

#[test]
fn stock_adjustments_add_subtract_and_never_go_below_zero() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 5.0)).unwrap();
    let stock = |tx: &chassis::storage::rusqlite::Transaction| {
        medicines::get(tx, id, today()).unwrap().stock
    };
    crate::store::stock::adjust_stock(&tx, id, 2.5).unwrap();
    assert_eq!(stock(&tx), 7.5);
    crate::store::stock::adjust_stock(&tx, id, -7.5).unwrap();
    assert_eq!(stock(&tx), 0.0);
    crate::store::stock::adjust_stock(&tx, id, -3.0).unwrap();
    assert_eq!(stock(&tx), 0.0, "the floor is zero");
    crate::store::stock::adjust_stock(&tx, id, 1.0).unwrap();
    assert_eq!(stock(&tx), 1.0, "and it recovers from the floor");
}

#[test]
fn taking_more_than_is_left_floors_the_stock_at_zero() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 1.0)).unwrap();
    intakes::record(&tx, &taken(id, "2026-11-01T08:00", 3.0)).unwrap();
    assert_eq!(medicines::get(&tx, id, today()).unwrap().stock, 0.0);
}

#[test]
fn list_puts_active_first_then_sorts_names_without_case() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    medicines::insert(&tx, &medicine("banana", 1.0)).unwrap();
    medicines::insert(&tx, &medicine("Apple", 1.0)).unwrap();
    let off = medicines::insert(&tx, &medicine("Aardvark", 1.0)).unwrap();
    medicines::set_active(&tx, off, false).unwrap();
    let names: Vec<_> = medicines::list(&tx, today())
        .unwrap()
        .into_iter()
        .map(|m| m.name)
        .collect();
    assert_eq!(names, ["Apple", "banana", "Aardvark"]);
}

#[test]
fn update_replaces_the_fields_and_the_symptom_links() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 1.0)).unwrap();
    let mut changed = medicine("A2", 8.0);
    changed.symptoms = vec!["fever".into(), "cough".into()];
    medicines::update(&tx, id, &changed).unwrap();
    let m = medicines::get(&tx, id, today()).unwrap();
    assert_eq!((m.name.as_str(), m.stock), ("A2", 8.0));
    assert_eq!(m.symptoms, ["cough", "fever"]);
    assert!(medicines::update(&tx, 999, &changed).is_err());
}

#[test]
fn unlinking_a_symptom_from_the_last_medicine_keeps_the_symptom() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 1.0)).unwrap();
    let mut none = medicine("A", 1.0);
    none.symptoms.clear();
    medicines::update(&tx, id, &none).unwrap();
    let all = symptoms::list(&tx).unwrap();
    assert_eq!(all.len(), 1);
    assert_eq!(all[0].medicines, 0);
}

#[test]
fn derived_judgements_come_from_the_row_and_today() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let mut input = medicine("A", 2.0);
    input.low_stock_at = Some(2.0);
    input.opened_on = Some("2026-10-01".into());
    input.shelf_life_days = Some(40);
    input.expires_on = Some("2027-06-01".into());
    input.course_ends_on = Some("2026-10-31".into());
    let id = medicines::insert(&tx, &input).unwrap();
    let m = medicines::get(&tx, id, today()).unwrap();
    assert_eq!(m.effective_expiry.as_deref(), Some("2026-11-10"));
    assert_eq!(m.stock_level, crate::domain::medicine::StockLevel::Low);
    assert!(m.course_over, "the course ended yesterday");
    assert_eq!(
        m.freshness,
        crate::domain::medicine::Freshness::ExpiringSoon
    );
}

#[test]
fn deleting_twice_or_an_unknown_id_is_not_found() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 1.0)).unwrap();
    medicines::delete(&tx, id).unwrap();
    assert!(medicines::delete(&tx, id).is_err());
    assert!(medicines::delete(&tx, 999).is_err());
}

#[test]
fn a_switched_off_medicine_still_lists_with_its_symptoms() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let on = medicines::insert(&tx, &medicine("On", 1.0)).unwrap();
    let off = medicines::insert(&tx, &medicine("Off", 1.0)).unwrap();
    let gone = medicines::insert(&tx, &medicine("Gone", 1.0)).unwrap();
    medicines::set_active(&tx, off, false).unwrap();
    medicines::delete(&tx, gone).unwrap();
    let rows = medicines::list(&tx, today()).unwrap();
    let with_headache: Vec<_> = rows
        .iter()
        .filter(|m| m.symptoms.iter().any(|s| s == "headache"))
        .map(|m| (m.id, m.active))
        .collect();
    assert_eq!(with_headache, [(on, true), (off, false)]);
}
