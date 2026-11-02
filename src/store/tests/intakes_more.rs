//! Intake log: ordering, day filter (including LIKE metacharacters), stock effects.

use super::*;

fn start(conn: &mut Connection) -> (chassis::storage::rusqlite::Transaction<'_>, i64) {
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 20.0)).unwrap();
    (tx, id)
}

#[test]
fn list_is_newest_first_and_honours_the_limit() {
    let mut conn = db();
    let (tx, id) = start(&mut conn);
    for at in ["2026-11-01T08:00", "2026-11-03T08:00", "2026-11-02T23:59"] {
        intakes::record(&tx, &taken(id, at, 1.0)).unwrap();
    }
    let all: Vec<_> = intakes::list(&tx, "", 10)
        .unwrap()
        .into_iter()
        .map(|i| i.taken_at)
        .collect();
    assert_eq!(
        all,
        ["2026-11-03T08:00", "2026-11-02T23:59", "2026-11-01T08:00"]
    );
    assert_eq!(intakes::list(&tx, "", 2).unwrap().len(), 2);
    assert_eq!(
        intakes::list(&tx, "", 2).unwrap()[0].taken_at,
        "2026-11-03T08:00"
    );
}

#[test]
fn the_day_filter_matches_one_day_only() {
    let mut conn = db();
    let (tx, id) = start(&mut conn);
    for at in [
        "2026-11-01T08:00",
        "2026-11-01T22:00",
        "2026-11-10T08:00",
        "2026-12-01T08:00",
    ] {
        intakes::record(&tx, &taken(id, at, 1.0)).unwrap();
    }
    assert_eq!(intakes::list(&tx, "2026-11-01", 50).unwrap().len(), 2);
    assert_eq!(intakes::list(&tx, "2026-11-10", 50).unwrap().len(), 1);
    assert!(intakes::list(&tx, "2026-11-02", 50).unwrap().is_empty());
}

#[test]
fn like_wildcards_in_the_day_are_literal_text() {
    let mut conn = db();
    let (tx, id) = start(&mut conn);
    intakes::record(&tx, &taken(id, "2026-11-01T08:00", 1.0)).unwrap();
    for hostile in ["%", "_", "2026-11-0_", "2026-%", "2026-11-01%", "\\"] {
        assert!(
            intakes::list(&tx, hostile, 50).unwrap().is_empty(),
            "'{hostile}' must not act as a wildcard"
        );
    }
}

#[test]
fn skipped_doses_are_logged_without_touching_the_stock() {
    let mut conn = db();
    let (tx, id) = start(&mut conn);
    let mut skip = taken(id, "2026-11-01T08:00", 2.0);
    skip.status = "skipped".into();
    let row = intakes::record(&tx, &skip).unwrap();
    assert_eq!(medicines::get(&tx, id, today()).unwrap().stock, 20.0);
    intakes::undo(&tx, row).unwrap();
    assert_eq!(medicines::get(&tx, id, today()).unwrap().stock, 20.0);
}

#[test]
fn editing_a_skipped_intake_leaves_the_stock_alone() {
    let mut conn = db();
    let (tx, id) = start(&mut conn);
    let mut skip = taken(id, "2026-11-01T08:00", 1.0);
    skip.status = "skipped".into();
    let row = intakes::record(&tx, &skip).unwrap();
    let edit = IntakeEdit {
        taken_at: "2026-11-01T09:00".into(),
        dose: 5.0,
        note: String::new(),
    };
    intakes::edit(&tx, row, &edit).unwrap();
    assert_eq!(medicines::get(&tx, id, today()).unwrap().stock, 20.0);
}

#[test]
fn unknown_statuses_and_medicines_are_refused() {
    let mut conn = db();
    let (tx, id) = start(&mut conn);
    let mut bad = taken(id, "2026-11-01T08:00", 1.0);
    bad.status = "maybe".into();
    assert!(intakes::record(&tx, &bad).is_err());
    assert!(intakes::record(&tx, &taken(999, "2026-11-01T08:00", 1.0)).is_err());
    assert!(intakes::list(&tx, "", 10).unwrap().is_empty());
}

#[test]
fn a_missing_dose_defaults_to_one_and_the_note_is_kept() {
    let mut conn = db();
    let (tx, id) = start(&mut conn);
    let mut input = taken(id, "2026-11-01T08:00", 0.0);
    input.dose = None;
    input.note = "with food".into();
    input.planned_for = Some("2026-11-01T08:00".into());
    intakes::record(&tx, &input).unwrap();
    let row = &intakes::list(&tx, "", 5).unwrap()[0];
    assert_eq!((row.dose, row.note.as_str()), (1.0, "with food"));
    assert_eq!(row.planned_for.as_deref(), Some("2026-11-01T08:00"));
    assert_eq!(medicines::get(&tx, id, today()).unwrap().stock, 19.0);
}

#[test]
fn undo_and_edit_of_unknown_ids_are_not_found() {
    let mut conn = db();
    let (tx, _) = start(&mut conn);
    assert!(intakes::undo(&tx, 404).is_err());
    let edit = IntakeEdit {
        taken_at: "2026-11-01T09:00".into(),
        dose: 1.0,
        note: String::new(),
    };
    assert!(intakes::edit(&tx, 404, &edit).is_err());
}

#[test]
fn recent_taken_minutes_ago_counts_back_across_days_and_skips_skipped_and_future() {
    let mut conn = db();
    let (tx, id) = start(&mut conn);
    for at in ["2026-11-01T07:30", "2026-10-31T23:00", "2026-11-01T12:00"] {
        intakes::record(&tx, &taken(id, at, 1.0)).unwrap();
    }
    let mut skip = taken(id, "2026-11-01T08:00", 1.0);
    skip.status = "skipped".into();
    intakes::record(&tx, &skip).unwrap();
    // "now" is 2026-11-01 09:00; the 12:00 entry lies in the future.
    let mut ago = intakes::recent_taken_minutes_ago(&tx, id, today().to_days(), 9 * 60).unwrap();
    ago.sort();
    assert_eq!(ago, [90, 600]);
}

#[test]
fn a_soft_deleted_medicine_cannot_be_logged_against() {
    let mut conn = db();
    let (tx, id) = start(&mut conn);
    medicines::delete(&tx, id).unwrap();
    assert!(intakes::record(&tx, &taken(id, "2026-11-01T08:00", 1.0)).is_err());
    medicines::restore(&tx, id).unwrap();
    assert!(intakes::list(&tx, "", 10).unwrap().is_empty());
}
