//! What the calendar hub was last told, per external id.

use std::collections::HashMap;

use chassis::Error;
use chassis::storage::rusqlite::{Connection, Transaction, params};

use super::db_err;

/// external id -> fingerprint of the event as last sent.
pub fn all(conn: &Connection) -> Result<HashMap<String, String>, Error> {
    let mut stmt = conn
        .prepare("SELECT external_id, fingerprint FROM agenda_sent")
        .map_err(db_err)?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))
        .map_err(db_err)?;
    rows.collect::<Result<_, _>>().map_err(db_err)
}

pub fn remember(tx: &Transaction, external_id: &str, fingerprint: &str) -> Result<(), Error> {
    tx.execute(
        "INSERT INTO agenda_sent (external_id, fingerprint) VALUES (?1, ?2)
         ON CONFLICT(external_id) DO UPDATE SET fingerprint = ?2, sent_at = datetime('now')",
        params![external_id, fingerprint],
    )
    .map_err(db_err)?;
    Ok(())
}

pub fn forget(tx: &Transaction, external_id: &str) -> Result<(), Error> {
    tx.execute(
        "DELETE FROM agenda_sent WHERE external_id = ?1",
        [external_id],
    )
    .map_err(db_err)?;
    Ok(())
}
