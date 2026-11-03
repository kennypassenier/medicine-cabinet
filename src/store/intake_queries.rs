//! Reading the intake log: the list, one intake by `uid`, the guard's
//! history, and the lookup an idempotent create needs.

use chassis::Error;
use chassis::storage::rusqlite::{Connection, OptionalExtension, Row, params};
use serde::Serialize;

use super::db_err;
use crate::domain::date::{Date, parse_minutes};

#[derive(Debug, Serialize)]
pub struct Intake {
    pub id: i64,
    /// Stable key the browser can make while offline.
    pub uid: String,
    pub rev: u64,
    pub medicine_id: i64,
    pub medicine_name: String,
    pub taken_at: String,
    pub planned_for: Option<String>,
    pub status: String,
    pub dose: f64,
    pub note: String,
}

const SELECT: &str = "SELECT i.id, i.medicine_id, m.name, i.taken_at, i.planned_for, i.status,
            i.dose, i.note, i.uid, i.rev
     FROM intakes i JOIN medicines m ON m.id = i.medicine_id";

fn read_row(r: &Row) -> chassis::storage::rusqlite::Result<Intake> {
    Ok(Intake {
        id: r.get(0)?,
        medicine_id: r.get(1)?,
        medicine_name: r.get(2)?,
        taken_at: r.get(3)?,
        planned_for: r.get(4)?,
        status: r.get(5)?,
        dose: r.get(6)?,
        note: r.get(7)?,
        uid: r.get(8)?,
        rev: super::sync_log::read_rev(r, 9)?,
    })
}

/// Intakes whose local time starts with `prefix` (a day, `YYYY-MM-DD`) or
/// all of them newest first when `prefix` is empty, capped at `limit`.
pub fn list(conn: &Connection, prefix: &str, limit: i64) -> Result<Vec<Intake>, Error> {
    let sql = format!(
        "{SELECT} WHERE m.deleted_at IS NULL AND substr(i.taken_at, 1, length(?1)) = ?1
         ORDER BY i.taken_at DESC LIMIT ?2"
    );
    let mut stmt = conn.prepare(&sql).map_err(db_err)?;
    let rows = stmt
        .query_map(params![prefix, limit], read_row)
        .map_err(db_err)?;
    rows.collect::<Result<_, _>>().map_err(db_err)
}

/// One intake by its `uid`, as the list shows it, for the offline feed.
pub fn get_by_uid(conn: &Connection, uid: &str) -> Result<Option<Intake>, Error> {
    let sql = format!("{SELECT} WHERE m.deleted_at IS NULL AND i.uid = ?1");
    conn.query_row(&sql, [uid], read_row)
        .optional()
        .map_err(db_err)
}

/// Taken-dose times of one medicine in the 48 hours before `now`, as
/// minutes before `now` (for the as-needed dosing guard).
pub fn recent_taken_minutes_ago(
    conn: &Connection,
    medicine_id: i64,
    now_days: i64,
    now_minutes: i64,
) -> Result<Vec<i64>, Error> {
    let mut stmt = conn
        .prepare(
            "SELECT taken_at FROM intakes WHERE medicine_id = ?1 AND status = 'taken'
                  ORDER BY taken_at DESC LIMIT 50",
        )
        .map_err(db_err)?;
    let rows = stmt
        .query_map([medicine_id], |r| r.get::<_, String>(0))
        .map_err(db_err)?;
    let mut out = Vec::new();
    let mut unreadable = 0;
    for row in rows {
        let text = row.map_err(db_err)?;
        // A row written before times were validated must not block the guard.
        let (Ok(day), Ok(minutes)) = (Date::parse(&text), parse_minutes(&text)) else {
            unreadable += 1;
            continue;
        };
        let ago = (now_days - day.to_days()) * 1440 + now_minutes - i64::from(minutes);
        if ago >= 0 {
            out.push(ago);
        }
    }
    if unreadable > 0 {
        eprintln!("dose guard: skipped {unreadable} intake(s) with an unreadable time");
    }
    Ok(out)
}

/// An intake the request's `uid` or `Idempotency-Key` already points at.
#[derive(Debug, Clone, PartialEq)]
pub struct Held {
    pub id: i64,
    pub uid: String,
    pub rev: u64,
    /// The `Idempotency-Key` it was created with, if one was sent then.
    pub idempotency_key: Option<String>,
}

/// The stored intake for a create request: by its `uid` when the browser
/// sent one, else by the `Idempotency-Key` alone.
pub fn find_held(
    conn: &Connection,
    uid: Option<&str>,
    idempotency_key: Option<&str>,
) -> Result<Option<Held>, Error> {
    let (column, value) = match (uid, idempotency_key) {
        (Some(uid), _) => ("uid", uid),
        (None, Some(key)) => ("idempotency_key", key),
        (None, None) => return Ok(None),
    };
    let sql = format!("SELECT id, uid, rev, idempotency_key FROM intakes WHERE {column} = ?1");
    conn.query_row(&sql, [value], |r| {
        Ok(Held {
            id: r.get(0)?,
            uid: r.get(1)?,
            rev: super::sync_log::read_rev(r, 2)?,
            idempotency_key: r.get(3)?,
        })
    })
    .optional()
    .map_err(db_err)
}

/// The row id a request path names: a number is the id itself, anything
/// else is an intake `uid` (the offline queue addresses intakes by `uid`).
pub fn resolve(conn: &Connection, reference: &str) -> Result<i64, Error> {
    if let Ok(id) = reference.parse::<i64>() {
        return Ok(id);
    }
    conn.query_row("SELECT id FROM intakes WHERE uid = ?1", [reference], |r| {
        r.get(0)
    })
    .optional()
    .map_err(db_err)?
    .ok_or_else(|| {
        Error::new(
            chassis::Kind::NotFound,
            "that intake does not exist",
            "reload the list; it may have been undone",
        )
    })
}
