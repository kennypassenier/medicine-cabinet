//! The reminder log and the agenda-sent record.

use super::*;
use crate::store::{agenda_sent, reminder_log};

fn key(medicine_id: i64, planned: &str, kind: &str) -> reminder_log::SentKey {
    (medicine_id, planned.to_string(), kind.to_string())
}

#[test]
fn marking_twice_reports_the_second_as_already_sent() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 1.0)).unwrap();
    let k = key(id, "2026-11-01T08:00", "due");
    assert!(reminder_log::mark(&tx, &k).unwrap());
    assert!(!reminder_log::mark(&tx, &k).unwrap());
    assert!(reminder_log::mark(&tx, &key(id, "2026-11-01T08:00", "overdue")).unwrap());
}

#[test]
fn marking_for_an_unknown_medicine_is_an_error() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    assert!(reminder_log::mark(&tx, &key(404, "2026-11-01T08:00", "due")).is_err());
}

#[test]
fn sent_on_returns_slots_and_digests_of_that_day_only() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 1.0)).unwrap();
    for (planned, kind) in [
        ("2026-11-01T08:00", "due"),
        ("2026-11-01", "low_stock"),
        ("2026-11-02T08:00", "due"),
        ("2026-10-31T08:00", "due"),
    ] {
        reminder_log::mark(&tx, &key(id, planned, kind)).unwrap();
    }
    let sent = reminder_log::sent_on(&tx, "2026-11-01").unwrap();
    assert_eq!(sent.len(), 2);
    assert!(sent.contains(&key(id, "2026-11-01", "low_stock")));
}

#[test]
fn sent_on_treats_wildcards_literally() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 1.0)).unwrap();
    reminder_log::mark(&tx, &key(id, "2026-11-01T08:00", "due")).unwrap();
    for hostile in ["%", "_", "2026-11-0_"] {
        assert!(
            reminder_log::sent_on(&tx, hostile).unwrap().is_empty(),
            "{hostile}"
        );
    }
}

#[test]
fn prune_removes_only_days_before_the_cutoff() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 1.0)).unwrap();
    for day in ["2026-10-30", "2026-10-31", "2026-11-01"] {
        reminder_log::mark(&tx, &key(id, &format!("{day}T08:00"), "due")).unwrap();
    }
    reminder_log::prune(&tx, "2026-10-31").unwrap();
    assert!(reminder_log::sent_on(&tx, "2026-10-30").unwrap().is_empty());
    assert_eq!(reminder_log::sent_on(&tx, "2026-10-31").unwrap().len(), 1);
    assert_eq!(reminder_log::sent_on(&tx, "2026-11-01").unwrap().len(), 1);
}

#[test]
fn log_lines_go_with_a_purged_medicine() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    let id = medicines::insert(&tx, &medicine("A", 1.0)).unwrap();
    reminder_log::mark(&tx, &key(id, "2026-11-01T08:00", "due")).unwrap();
    medicines::delete(&tx, id).unwrap();
    tx.execute(
        "UPDATE medicines SET deleted_at = datetime('now','-20 days')",
        [],
    )
    .unwrap();
    assert_eq!(medicines::purge_deleted(&tx).unwrap(), 1);
    assert!(reminder_log::sent_on(&tx, "2026-11-01").unwrap().is_empty());
}

#[test]
fn agenda_sent_remembers_overwrites_and_forgets() {
    let mut conn = db();
    let tx = conn.transaction().unwrap();
    assert!(agenda_sent::all(&tx).unwrap().is_empty());
    agenda_sent::remember(&tx, "expiry:1", "fp1").unwrap();
    agenda_sent::remember(&tx, "reorder:1", "fpA").unwrap();
    agenda_sent::remember(&tx, "expiry:1", "fp2").unwrap();
    let all = agenda_sent::all(&tx).unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all["expiry:1"], "fp2");
    agenda_sent::forget(&tx, "expiry:1").unwrap();
    agenda_sent::forget(&tx, "never-there").unwrap();
    let left = agenda_sent::all(&tx).unwrap();
    assert_eq!(left.len(), 1);
    assert!(left.contains_key("reorder:1"));
}
