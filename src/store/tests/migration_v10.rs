//! Migration V10 gives every existing row of the four kinds its own uid.

use chassis::storage::rusqlite::Connection;
use chassis::storage::sync::client_key;

use crate::schema::ALL_SQL;

#[test]
fn existing_rows_get_distinct_well_formed_uids() {
    let conn = Connection::open_in_memory().unwrap();
    // V10 is the second-to-last step; V11 (desc_rev) comes after it.
    let (before, from_v10) = ALL_SQL.split_at(ALL_SQL.len() - 2);
    for sql in before {
        conn.execute_batch(sql).unwrap();
    }
    conn.execute_batch(
        "INSERT INTO medicines (name) VALUES ('A'), ('B'), ('C');
         INSERT INTO symptoms (name) VALUES ('x'), ('y');
         INSERT INTO symptom_categories (name) VALUES ('Head'), ('Gut');",
    )
    .unwrap();
    conn.execute_batch(from_v10[0]).unwrap();
    for table in [
        "medicines",
        "symptoms",
        "symptom_categories",
        "medicine_forms",
    ] {
        let uids: Vec<String> = conn
            .prepare(&format!("SELECT uid FROM {table}"))
            .unwrap()
            .query_map([], |r| r.get(0))
            .unwrap()
            .collect::<Result<_, _>>()
            .unwrap();
        assert!(!uids.is_empty(), "{table}");
        for uid in &uids {
            assert!(client_key(uid).is_ok(), "{table}: {uid}");
        }
        let mut unique = uids.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), uids.len(), "{table} uids are distinct");
    }
}
