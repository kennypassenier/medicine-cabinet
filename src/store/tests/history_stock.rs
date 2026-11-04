//! Logged doses outlive a purged medicine, and stock arithmetic is exact.

use chassis::storage::rusqlite::Connection;

use super::*;
use crate::domain::quantity;
use crate::store::stock;
use crate::testkit::logged;

fn purge_window_passed(tx: &chassis::storage::rusqlite::Transaction, id: i64) {
    tx.execute(
        "UPDATE medicines SET deleted_at = datetime('now', '-20 days') WHERE id = ?1",
        [id],
    )
    .unwrap();
}

#[test]
fn a_purged_medicines_doses_stay_with_the_stored_name() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("Amoxicillin", 30.0)).unwrap();
    for day in 1..=3 {
        intakes::record(&tx, &taken(id, &format!("2026-11-0{day}T08:00"), 1.0)).unwrap();
    }
    medicines::delete(&tx, id).unwrap();
    assert!(
        intakes::list(&tx, "", 50).unwrap().is_empty(),
        "hidden while deleted"
    );
    purge_window_passed(&tx, id);
    assert_eq!(medicines::purge_deleted(&tx).unwrap(), 1);

    let kept = intakes::list(&tx, "", 50).unwrap();
    assert_eq!(kept.len(), 3, "the doses survive the purge");
    assert!(kept.iter().all(|i| i.medicine_id.is_none()));
    assert!(kept.iter().all(|i| i.medicine_name == "Amoxicillin"));
    let json = serde_json::to_value(&kept[0]).unwrap();
    assert!(json["medicine_id"].is_null(), "no link to a medicine");
}

#[test]
fn purging_records_the_doses_as_live_never_as_deleted() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("Amoxicillin", 5.0)).unwrap();
    intakes::record(&tx, &taken(id, "2026-11-01T08:00", 1.0)).unwrap();
    let uid = intakes::list(&tx, "", 1).unwrap().remove(0).uid;
    medicines::delete(&tx, id).unwrap();
    assert!(
        logged(&tx, "intakes", &uid).unwrap().1,
        "hidden with the medicine"
    );
    purge_window_passed(&tx, id);
    medicines::purge_deleted(&tx).unwrap();
    assert!(
        !logged(&tx, "intakes", &uid).unwrap().1,
        "visible again as history"
    );
    assert!(intakes::get_by_uid(&tx, &uid).unwrap().is_some());
}

#[test]
fn renaming_a_medicine_updates_the_stored_name() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("Old", 5.0)).unwrap();
    intakes::record(&tx, &taken(id, "2026-11-01T08:00", 1.0)).unwrap();
    medicines::update(&tx, id, &medicine("New", 4.0), true).unwrap();
    medicines::delete(&tx, id).unwrap();
    purge_window_passed(&tx, id);
    medicines::purge_deleted(&tx).unwrap();
    assert_eq!(intakes::list(&tx, "", 5).unwrap()[0].medicine_name, "New");
}

#[test]
fn editing_or_undoing_an_orphaned_intake_moves_no_stock() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("Gone", 5.0)).unwrap();
    let other = medicines::insert(&tx, &medicine("Other", 5.0)).unwrap();
    intakes::record(&tx, &taken(id, "2026-11-01T08:00", 2.0)).unwrap();
    medicines::delete(&tx, id).unwrap();
    purge_window_passed(&tx, id);
    medicines::purge_deleted(&tx).unwrap();
    let orphan = intakes::list(&tx, "", 5).unwrap().remove(0);

    let edit = IntakeEdit {
        taken_at: "2026-11-01T09:00".into(),
        dose: 4.0,
        note: "x".into(),
    };
    intakes::edit(&tx, orphan.id, &edit).unwrap();
    let after = intakes::list(&tx, "", 5).unwrap().remove(0);
    assert_eq!(
        (after.dose, after.taken_at.as_str()),
        (4.0, "2026-11-01T09:00")
    );
    assert_eq!(
        stock::hundredths(&tx, other).unwrap(),
        500,
        "other stock untouched"
    );
    intakes::undo(&tx, orphan.id).unwrap();
    assert!(intakes::list(&tx, "", 5).unwrap().is_empty());
    assert_eq!(stock::hundredths(&tx, other).unwrap(), 500);
}

#[test]
fn three_tenths_from_one_leave_exactly_seventy_hundredths() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("Syrup", 1.0)).unwrap();
    for minute in 0..3 {
        intakes::record(&tx, &taken(id, &format!("2026-11-01T08:0{minute}"), 0.1)).unwrap();
    }
    assert_eq!(stock::hundredths(&tx, id).unwrap(), 70);
    assert_eq!(medicines::get(&tx, id, today()).unwrap().stock, 0.7);
    stock::adjust_stock(&tx, id, 0.1).unwrap();
    stock::adjust_stock(&tx, id, 0.2).unwrap();
    assert_eq!(medicines::get(&tx, id, today()).unwrap().stock, 1.0);
}

#[test]
fn undoing_gives_back_exactly_what_was_taken() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("Syrup", 0.25)).unwrap();
    let intake = intakes::record(&tx, &taken(id, "2026-11-01T08:00", 0.4)).unwrap();
    assert_eq!(
        stock::hundredths(&tx, id).unwrap(),
        0,
        "only 0.25 was there to take"
    );
    intakes::undo(&tx, intake).unwrap();
    assert_eq!(
        stock::hundredths(&tx, id).unwrap(),
        quantity::to_hundredths(0.25)
    );
}

#[test]
fn the_migration_converts_stock_and_snapshots_names() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA foreign_keys = ON;").unwrap();
    for sql in &crate::schema::ALL_SQL[..8] {
        conn.execute_batch(sql).unwrap();
    }
    conn.execute_batch(
        "INSERT INTO medicines (id, name, stock, low_stock_at) VALUES (1, 'Old', 0.7000000000000001, 2.5);
         INSERT INTO intakes (id, medicine_id, taken_at, status, dose, deducted, uid, rev, idempotency_key)
             VALUES (1, 1, '2026-11-01T08:00', 'taken', 0.1, NULL, 'u-1', 4, 'k-1');",
    )
    .unwrap();
    conn.execute_batch(crate::schema::ALL_SQL[8]).unwrap();
    let (stock, low): (i64, i64) = conn
        .query_row(
            "SELECT stock_hundredths, low_stock_at_hundredths FROM medicines",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!((stock, low), (70, 250));
    let row: (String, i64, Option<i64>, i64, String, String) = conn
        .query_row(
            "SELECT medicine_name, dose_hundredths, deducted_hundredths, rev, uid, idempotency_key FROM intakes",
            [],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?)),
        )
        .unwrap();
    assert_eq!(row, ("Old".into(), 10, None, 4, "u-1".into(), "k-1".into()));
    conn.execute("DELETE FROM medicines WHERE id = 1", [])
        .unwrap();
    let left: (i64, Option<i64>) = conn
        .query_row("SELECT count(*), max(medicine_id) FROM intakes", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!(left, (1, None), "the FK now sets NULL instead of cascading");
}
