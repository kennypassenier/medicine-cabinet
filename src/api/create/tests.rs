use super::*;
use crate::store::forms;
use crate::testkit::sync_db;

const UID: &str = "3f2b8c1e-5a4d-4e0a-9b1c-2d7e6f8a9b0c";

fn key(s: &str) -> Option<Uuid> {
    Some(Uuid::parse_str(s).unwrap())
}

const K1: &str = "0b9d1a52-7c1e-4d38-8a55-1f0e2d3c4b5a";
const K2: &str = "11111111-7c1e-4d38-8a55-1f0e2d3c4b5a";

fn make_form(
    tx: &Transaction,
    uid: Option<&str>,
    sent: Option<Uuid>,
) -> Result<Created, KeyExists> {
    keyed(tx, Coll::Forms, uid, sent, |tx, keys| {
        forms::create_keyed(tx, "Gel", "g", &[], keys)
    })
    .unwrap()
}

fn form_count(tx: &Transaction) -> i64 {
    tx.query_row(
        "SELECT COUNT(*) FROM medicine_forms WHERE name = 'Gel'",
        [],
        |r| r.get(0),
    )
    .unwrap()
}

#[test]
fn a_new_uid_creates_under_that_uid() {
    let mut conn = sync_db();
    let tx = conn.transaction().unwrap();
    let made = make_form(&tx, Some(UID), key(K1)).unwrap();
    assert_eq!(made.uid, UID);
    assert!(made.rev >= 1);
}

#[test]
fn the_same_key_replays_without_a_second_row() {
    let mut conn = sync_db();
    let tx = conn.transaction().unwrap();
    let first = make_form(&tx, Some(UID), key(K1)).unwrap();
    let again = make_form(&tx, Some(UID), key(K1)).unwrap();
    assert_eq!(first, again);
    assert_eq!(form_count(&tx), 1);
}

#[test]
fn a_held_uid_with_another_key_is_refused() {
    let mut conn = sync_db();
    let tx = conn.transaction().unwrap();
    make_form(&tx, Some(UID), key(K1)).unwrap();
    assert!(make_form(&tx, Some(UID), key(K2)).is_err());
    assert!(make_form(&tx, Some(UID), None).is_err());
    assert_eq!(form_count(&tx), 1);
}

#[test]
fn without_a_client_uid_the_server_makes_one_and_the_key_still_replays() {
    let mut conn = sync_db();
    let tx = conn.transaction().unwrap();
    let first = make_form(&tx, None, key(K1)).unwrap();
    assert_eq!(first.uid.len(), 36);
    assert_eq!(make_form(&tx, None, key(K1)).unwrap(), first);
    assert_eq!(form_count(&tx), 1);
}

#[test]
fn a_client_id_must_be_a_uuid() {
    assert!(client_uid(Some("12")).is_err());
    assert_eq!(client_uid(None).unwrap(), None);
    assert_eq!(
        client_uid(Some(&UID.to_uppercase())).unwrap().as_deref(),
        Some(UID)
    );
}
