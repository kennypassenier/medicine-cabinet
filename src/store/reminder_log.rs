//! The record of reminders already sent.

use std::collections::HashSet;

use chassis::Error;
use chassis::storage::rusqlite::{Connection, Transaction, params};

use super::db_err;

/// `(medicine id, planned_for, kind)` of a reminder already sent.
pub type SentKey = (i64, String, String);

/// Everything sent for slots or digests on `day` (`YYYY-MM-DD`).
pub fn sent_on(conn: &Connection, day: &str) -> Result<HashSet<SentKey>, Error> {
    let mut stmt = conn
        .prepare("SELECT medicine_id, planned_for, kind FROM reminder_log WHERE planned_for LIKE ?1 || '%'")
        .map_err(db_err)?;
    let rows = stmt
        .query_map([day], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .map_err(db_err)?;
    rows.collect::<Result<_, _>>().map_err(db_err)
}

/// Mark as sent; false when it was already marked (another tick won).
pub fn mark(tx: &Transaction, key: &SentKey) -> Result<bool, Error> {
    let added = tx
        .execute(
            "INSERT OR IGNORE INTO reminder_log (medicine_id, planned_for, kind) VALUES (?1, ?2, ?3)",
            params![key.0, key.1, key.2],
        )
        .map_err(db_err)?;
    Ok(added > 0)
}

/// Forget log lines older than `before_day`, so the table never grows.
pub fn prune(tx: &Transaction, before_day: &str) -> Result<(), Error> {
    tx.execute(
        "DELETE FROM reminder_log WHERE planned_for < ?1",
        [before_day],
    )
    .map_err(db_err)?;
    Ok(())
}
