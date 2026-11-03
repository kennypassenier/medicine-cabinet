//! The offline feed's change log: every store write records the rows it
//! touched here, in the same transaction, so the browser's local database
//! can follow (`GET /api/sync/<collection>`).
//!
//! A row is served while it is live. A row that is soft-deleted (a deleted
//! medicine and, with it, its intakes) or removed is recorded as a delete, so
//! it disappears from the client list; restoring it records an upsert.

use chassis::Error;
use chassis::storage::rusqlite::{OptionalExtension, Row, Transaction};
use chassis::storage::sync::SyncWrite;

use super::db_err;

/// One synced collection, which is one table of the app database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Coll {
    Medicines,
    Intakes,
    Symptoms,
    Categories,
    Forms,
}

/// Every collection, in the order they are declared to the kit.
pub const ALL: [Coll; 5] = [
    Coll::Medicines,
    Coll::Intakes,
    Coll::Symptoms,
    Coll::Categories,
    Coll::Forms,
];

impl Coll {
    /// The collection name the page knows.
    pub fn name(self) -> &'static str {
        match self {
            Coll::Medicines => "medicines",
            Coll::Intakes => "intakes",
            Coll::Symptoms => "symptoms",
            Coll::Categories => "categories",
            Coll::Forms => "forms",
        }
    }

    pub fn table(self) -> &'static str {
        match self {
            Coll::Medicines => "medicines",
            Coll::Intakes => "intakes",
            Coll::Symptoms => "symptoms",
            Coll::Categories => "symptom_categories",
            Coll::Forms => "medicine_forms",
        }
    }

    /// SQL for the item's key as text: the id, or the `uid` of an intake.
    pub fn key_expr(self) -> &'static str {
        match self {
            Coll::Intakes => "uid",
            _ => "CAST(id AS TEXT)",
        }
    }

    /// SQL that is true while the row must not be served.
    pub fn gone_expr(self) -> &'static str {
        match self {
            Coll::Medicines => "deleted_at IS NOT NULL",
            // An intake of a deleted medicine is hidden with it.
            Coll::Intakes => {
                "COALESCE((SELECT m.deleted_at IS NOT NULL FROM medicines m \
                 WHERE m.id = intakes.medicine_id), 1)"
            }
            _ => "0",
        }
    }
}

/// Read a `rev` column (SQLite stores it signed; revisions count up from 1).
pub fn read_rev(r: &Row, idx: usize) -> chassis::storage::rusqlite::Result<u64> {
    r.get::<_, i64>(idx).map(|v| v as u64)
}

/// A newly inserted row: record it at its starting revision.
pub fn created(tx: &Transaction, coll: Coll, id: i64) -> Result<(), Error> {
    record(tx, coll, id, false).map(|_| ())
}

/// A changed row: bump its revision and record it. Returns the new revision,
/// or `None` when the row does not exist (nothing is recorded then).
pub fn touch(tx: &Transaction, coll: Coll, id: i64) -> Result<Option<u64>, Error> {
    bump(tx, coll, id, false)
}

/// A row about to be removed for good: call it BEFORE the `DELETE`, so the
/// key and the revision can still be read.
pub fn tombstone(tx: &Transaction, coll: Coll, id: i64) -> Result<(), Error> {
    bump(tx, coll, id, true).map(|_| ())
}

/// [`touch`] for several rows (a missing one is skipped).
pub fn touch_all(tx: &Transaction, coll: Coll, ids: &[i64]) -> Result<(), Error> {
    for id in ids {
        touch(tx, coll, *id)?;
    }
    Ok(())
}

/// [`touch`] for an optional reference such as a category or form id.
pub fn touch_opt(tx: &Transaction, coll: Coll, id: Option<i64>) -> Result<(), Error> {
    id.map_or(Ok(()), |id| touch(tx, coll, id).map(|_| ()))
}

/// The intakes of a medicine (their body carries its name; a deleted
/// medicine hides them).
pub fn touch_intakes_of(tx: &Transaction, medicine_id: i64) -> Result<(), Error> {
    let ids = ids(
        tx,
        "SELECT id FROM intakes WHERE medicine_id = ?1",
        medicine_id,
    )?;
    touch_all(tx, Coll::Intakes, &ids)
}

/// The stored revision of a row (soft-deleted ones included), for the
/// `If-Match` check; `None` when there is no such row.
pub fn current_rev(tx: &Transaction, coll: Coll, id: i64) -> Result<Option<u64>, Error> {
    let sql = format!("SELECT rev FROM {} WHERE id = ?1", coll.table());
    let rev: Option<i64> = tx
        .query_row(&sql, [id], |r| r.get(0))
        .optional()
        .map_err(db_err)?;
    Ok(rev.map(|r| r as u64))
}

/// The revision of the last change recorded for a key: the answer to a
/// delete, whose row is gone.
pub fn logged_rev(tx: &Transaction, coll: Coll, key: &str) -> Result<Option<u64>, Error> {
    let rev: Option<i64> = tx
        .query_row(
            "SELECT rev FROM chassis_sync WHERE collection = ?1 AND key = ?2",
            [coll.name(), key],
            |r| r.get(0),
        )
        .optional()
        .map_err(db_err)?;
    Ok(rev.map(|r| r as u64))
}

/// A column of ids, for the lookups that decide what else a write touched.
pub fn ids(tx: &Transaction, sql: &str, arg: i64) -> Result<Vec<i64>, Error> {
    let mut stmt = tx.prepare(sql).map_err(db_err)?;
    let rows = stmt
        .query_map([arg], |r| r.get::<_, i64>(0))
        .map_err(db_err)?;
    rows.collect::<Result<_, _>>().map_err(db_err)
}

fn bump(tx: &Transaction, coll: Coll, id: i64, force_gone: bool) -> Result<Option<u64>, Error> {
    let sql = format!("UPDATE {} SET rev = rev + 1 WHERE id = ?1", coll.table());
    if tx.execute(&sql, [id]).map_err(db_err)? == 0 {
        return Ok(None);
    }
    record(tx, coll, id, force_gone)
}

/// Write the row's current state into the change log.
fn record(tx: &Transaction, coll: Coll, id: i64, force_gone: bool) -> Result<Option<u64>, Error> {
    let sql = format!(
        "SELECT rev, {}, {} FROM {} WHERE id = ?1",
        coll.key_expr(),
        coll.gone_expr(),
        coll.table()
    );
    let row: Option<(i64, String, bool)> = tx
        .query_row(&sql, [id], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .optional()
        .map_err(db_err)?;
    let Some((rev, key, gone)) = row else {
        return Ok(None);
    };
    let rev = rev as u64;
    if gone || force_gone {
        tx.sync_delete(coll.name(), &key, rev)?;
    } else {
        tx.sync_upsert(coll.name(), &key, rev)?;
    }
    Ok(Some(rev))
}

/// Record every live row the change log does not know yet. Runs at start:
/// rows that existed before the offline feed (and the forms a migration
/// seeds) have no entry, and migrations run before the log exists.
pub fn backfill(tx: &Transaction) -> Result<usize, Error> {
    let mut recorded = 0;
    for coll in ALL {
        let sql = format!(
            "SELECT id FROM {t} WHERE NOT ({gone}) AND {key} NOT IN \
             (SELECT key FROM chassis_sync WHERE collection = '{name}')",
            t = coll.table(),
            gone = coll.gone_expr(),
            key = coll.key_expr(),
            name = coll.name()
        );
        let mut stmt = tx.prepare(&sql).map_err(db_err)?;
        let found = stmt
            .query_map([], |r| r.get::<_, i64>(0))
            .map_err(db_err)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(db_err)?;
        drop(stmt);
        for id in &found {
            created(tx, coll, *id)?;
        }
        recorded += found.len();
    }
    Ok(recorded)
}
