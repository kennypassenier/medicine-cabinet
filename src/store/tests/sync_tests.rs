//! Every write records its change in the offline feed's log, in the same
//! transaction, and bumps the revision.

use super::*;
use crate::schema::UID_SQL;
use crate::store::refs;
use crate::store::sync_log::{self, Coll};
use crate::testkit::logged;
use chassis::storage::sync::client_key;

/// The sync key of a row: its uid.
fn key_of(tx: &chassis::storage::rusqlite::Transaction, coll: Coll, id: i64) -> String {
    refs::uid_of(tx, coll, id).unwrap()
}

#[test]
fn the_sql_uid_is_a_well_formed_uuid() {
    let conn = db();
    for _ in 0..20 {
        let uid: String = conn
            .query_row(&format!("SELECT {UID_SQL}"), [], |r| r.get(0))
            .unwrap();
        assert_eq!(uid.len(), 36);
        assert!(client_key(&uid).is_ok(), "{uid}");
    }
}

#[test]
fn a_medicine_is_logged_and_its_revision_grows_with_every_write() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 5.0)).unwrap();
    let key = key_of(&tx, Coll::Medicines, id);
    assert_eq!(logged(&tx, "medicines", &key), Some((1, false)));
    medicines::set_active(&tx, id, false).unwrap();
    assert_eq!(logged(&tx, "medicines", &key), Some((2, false)));
    crate::store::stock::adjust_stock(&tx, id, 2.0).unwrap();
    assert_eq!(logged(&tx, "medicines", &key), Some((3, false)));
}

#[test]
fn a_soft_delete_is_a_feed_delete_and_a_restore_brings_it_back() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 5.0)).unwrap();
    let uid = intakes::record_keyed(&tx, &taken(id, "2026-11-01T08:00", 1.0), None, None)
        .unwrap()
        .uid;
    medicines::delete(&tx, id).unwrap();
    assert!(
        logged(&tx, "medicines", &key_of(&tx, Coll::Medicines, id))
            .unwrap()
            .1
    );
    assert!(
        logged(&tx, "intakes", &uid).unwrap().1,
        "its intakes hide too"
    );
    medicines::restore(&tx, id).unwrap();
    assert!(
        !logged(&tx, "medicines", &key_of(&tx, Coll::Medicines, id))
            .unwrap()
            .1
    );
    assert!(!logged(&tx, "intakes", &uid).unwrap().1);
}

#[test]
fn a_taken_dose_moves_the_stock_and_logs_both_rows_and_undo_tombstones_the_intake() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 5.0)).unwrap();
    let before = logged(&tx, "medicines", &key_of(&tx, Coll::Medicines, id))
        .unwrap()
        .0;
    let rec = intakes::record_keyed(&tx, &taken(id, "2026-11-01T08:00", 1.0), None, None).unwrap();
    assert_eq!(rec.rev, 1);
    assert!(
        logged(&tx, "medicines", &key_of(&tx, Coll::Medicines, id))
            .unwrap()
            .0
            > before
    );
    intakes::undo(&tx, rec.id).unwrap();
    assert!(logged(&tx, "intakes", &rec.uid).unwrap().1);
}

#[test]
fn a_client_uid_is_kept_and_found_again_with_its_idempotency_key() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 5.0)).unwrap();
    let uid = "3f2b8c1e-5a4d-4e0a-9b1c-2d7e6f8a9b0c";
    let key = "0b9d1a52-7c1e-4d38-8a55-1f0e2d3c4b5a";
    let input = taken(id, "2026-11-01T08:00", 1.0);
    let rec = intakes::record_keyed(&tx, &input, Some(uid), Some(key)).unwrap();
    assert_eq!(rec.uid, uid);
    let held = intakes::find_held(&tx, Some(uid), None).unwrap().unwrap();
    assert_eq!(held.idempotency_key.as_deref(), Some(key));
    let by_key = intakes::find_held(&tx, None, Some(key)).unwrap().unwrap();
    assert_eq!(by_key.id, rec.id);
    assert!(intakes::find_held(&tx, None, None).unwrap().is_none());
}

#[test]
fn deleting_a_symptom_category_or_form_leaves_a_tombstone_and_touches_what_showed_it() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let cat = categories::create(&tx, "Head", &[]).unwrap();
    let sym = symptoms::create(&tx, "headache", Some(cat), &[]).unwrap();
    let (cat_key, sym_key) = (
        key_of(&tx, Coll::Categories, cat),
        key_of(&tx, Coll::Symptoms, sym),
    );
    let cat_rev = logged(&tx, "categories", &cat_key).unwrap().0;
    symptoms::delete(&tx, sym).unwrap();
    assert!(logged(&tx, "symptoms", &sym_key).unwrap().1);
    assert!(logged(&tx, "categories", &cat_key).unwrap().0 > cat_rev);
    categories::delete(&tx, cat).unwrap();
    assert!(logged(&tx, "categories", &cat_key).unwrap().1);
    let form = forms::create(&tx, "Gel", "g", &[]).unwrap();
    let form_key = key_of(&tx, Coll::Forms, form);
    forms::delete(&tx, form).unwrap();
    assert!(logged(&tx, "forms", &form_key).unwrap().1);
}

#[test]
fn backfill_logs_rows_that_predate_the_feed_once() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    // The forms seeded by a migration have no log entry yet.
    let seeded = sync_log::backfill(&tx).unwrap();
    assert!(seeded >= 9, "{seeded}");
    assert_eq!(sync_log::backfill(&tx).unwrap(), 0, "idempotent");
    assert!(
        logged(&tx, "forms", &key_of(&tx, Coll::Forms, 1)).is_some(),
        "a seeded form is served"
    );
    assert_eq!(sync_log::current_rev(&tx, Coll::Forms, 1).unwrap(), Some(1));
}
