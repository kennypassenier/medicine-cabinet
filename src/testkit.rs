//! Shared fixtures for unit tests: one fully filled `Medicine` to adjust.

use chassis::storage::rusqlite::Connection;

use crate::domain::medicine::{Freshness, StockLevel};
use crate::domain::schedule::SlotInput;
use crate::store::medicines::Medicine;

/// An active, scheduled, in-stock medicine with one daily 08:00 slot.
pub fn medicine(id: i64, name: &str) -> Medicine {
    Medicine {
        id,
        uid: format!("00000000-0000-4000-8000-{id:012}"),
        rev: 1,
        name: name.into(),
        purpose: String::new(),
        form: String::new(),
        form_uid: None,
        strength: String::new(),
        notes: String::new(),
        active: true,
        as_needed: false,
        min_gap_minutes: None,
        max_per_day: None,
        stock: 30.0,
        stock_unit: "capsules".into(),
        low_stock_at: None,
        expires_on: None,
        opened_on: None,
        shelf_life_days: None,
        course_ends_on: None,
        symptoms: vec![],
        slots: vec![SlotInput {
            time: "08:00".into(),
            days: 127,
            dose: 1.0,
        }],
        effective_expiry: None,
        freshness: Freshness::Fresh,
        stock_level: StockLevel::Ok,
        course_over: false,
    }
}

/// The kit creates these two tables when it opens the app file; unit tests
/// that open an in-memory database build them by hand (same columns).
const KIT_TABLES: &str = "
CREATE TABLE chassis_meta (key TEXT PRIMARY KEY, value TEXT NOT NULL) STRICT;
CREATE TABLE chassis_sync (
    collection TEXT NOT NULL, scope TEXT NOT NULL, key TEXT NOT NULL,
    seq INTEGER NOT NULL, rev INTEGER NOT NULL, deleted INTEGER NOT NULL DEFAULT 0,
    changed_at INTEGER NOT NULL, PRIMARY KEY (collection, scope, key)
) STRICT;
";

/// An in-memory database with the app's migrations and the kit's change-log tables.
pub fn sync_db() -> Connection {
    let conn = Connection::open_in_memory().expect("in-memory database");
    conn.execute_batch("PRAGMA foreign_keys = ON;")
        .expect("pragma");
    conn.execute_batch(KIT_TABLES).expect("kit tables");
    for sql in crate::schema::ALL_SQL {
        conn.execute_batch(sql).expect("migration applies");
    }
    conn
}

/// What the change log holds for an item: `(rev, deleted)`.
pub fn logged(conn: &Connection, collection: &str, key: &str) -> Option<(i64, bool)> {
    conn.query_row(
        "SELECT rev, deleted FROM chassis_sync WHERE collection = ?1 AND key = ?2",
        [collection, key],
        |r| Ok((r.get(0)?, r.get::<_, i64>(1)? != 0)),
    )
    .ok()
}
