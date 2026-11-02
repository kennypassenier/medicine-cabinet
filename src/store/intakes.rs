//! The intake log: what was taken or skipped, and when.

use chassis::Error;
use chassis::storage::rusqlite::{Connection, Transaction, params};
use serde::{Deserialize, Serialize};

use super::{db_err, not_found, stock};
use crate::domain::date::{Date, parse_datetime, parse_minutes};
use crate::domain::limits;

#[derive(Debug, Deserialize)]
pub struct IntakeInput {
    pub medicine_id: i64,
    /// Local `YYYY-MM-DDTHH:MM` when it happened.
    pub taken_at: String,
    /// The slot this answers, same format, when it was a scheduled dose.
    pub planned_for: Option<String>,
    #[serde(default = "taken")]
    pub status: String,
    pub dose: Option<f64>,
    #[serde(default)]
    pub note: String,
}

fn taken() -> String {
    "taken".into()
}

#[derive(Debug, Serialize)]
pub struct Intake {
    pub id: i64,
    pub medicine_id: i64,
    pub medicine_name: String,
    pub taken_at: String,
    pub planned_for: Option<String>,
    pub status: String,
    pub dose: f64,
    pub note: String,
}

/// The columns every intake lookup needs.
type Existing = (i64, String, f64, f64);

fn existing(tx: &Transaction, id: i64) -> Result<Existing, Error> {
    tx.query_row(
        "SELECT medicine_id, status, dose, COALESCE(deducted, dose) FROM intakes WHERE id = ?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
    )
    .map_err(|e| match e {
        chassis::storage::rusqlite::Error::QueryReturnedNoRows => not_found("intake", id),
        other => db_err(other),
    })
}

/// Record an intake; a taken dose also takes from the stock (never below zero).
pub fn record(tx: &Transaction, i: &IntakeInput) -> Result<i64, Error> {
    if i.status != "taken" && i.status != "skipped" {
        return Err(Error::invalid(
            format!(
                "status '{}' is not known",
                i.status.chars().take(40).collect::<String>()
            ),
            "use 'taken' or 'skipped'",
        ));
    }
    let taken_at = parse_datetime(&i.taken_at)?;
    let planned_for = i.planned_for.as_deref().map(parse_datetime).transpose()?;
    let note = limits::text("the note", &i.note, limits::NOTE_MAX, false)?;
    let dose = limits::amount("the dose", i.dose.unwrap_or(1.0), true)?;
    // A soft-deleted medicine is gone for logging purposes; a real database
    // failure must surface rather than read as "no such medicine".
    let exists = match tx.query_row(
        "SELECT 1 FROM medicines WHERE id = ?1 AND deleted_at IS NULL",
        [i.medicine_id],
        |_| Ok(()),
    ) {
        Ok(()) => true,
        Err(chassis::storage::rusqlite::Error::QueryReturnedNoRows) => false,
        Err(e) => return Err(db_err(e)),
    };
    if !exists {
        return Err(not_found("medicine", i.medicine_id));
    }
    if let Some(slot) = &planned_for {
        refuse_if_answered(tx, i.medicine_id, slot)?;
    }
    let deducted = if i.status == "taken" {
        stock::take_stock(tx, i.medicine_id, dose)?
    } else {
        0.0
    };
    tx.execute(
        "INSERT INTO intakes (medicine_id, taken_at, planned_for, status, dose, note, deducted)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            i.medicine_id,
            taken_at,
            planned_for,
            i.status,
            dose,
            note,
            deducted
        ],
    )
    .map_err(db_err)?;
    Ok(tx.last_insert_rowid())
}

/// A scheduled dose is answered once; a double tap must not take twice.
fn refuse_if_answered(tx: &Transaction, medicine_id: i64, slot: &str) -> Result<(), Error> {
    let answered = tx
        .query_row(
            "SELECT 1 FROM intakes WHERE medicine_id = ?1 AND planned_for = ?2",
            params![medicine_id, slot],
            |_| Ok(()),
        )
        .is_ok();
    if answered {
        return Err(Error::invalid(
            "this scheduled dose is already logged",
            "reload the app; undo the logged dose first if you want to log it again",
        ));
    }
    Ok(())
}

/// What can change on a logged intake.
#[derive(Debug, Deserialize)]
pub struct IntakeEdit {
    /// Local `YYYY-MM-DDTHH:MM`.
    pub taken_at: String,
    pub dose: f64,
    #[serde(default)]
    pub note: String,
}

/// Correct the time, dose or note of an intake; a changed dose moves the stock.
pub fn edit(tx: &Transaction, id: i64, e: &IntakeEdit) -> Result<(), Error> {
    let dose = limits::amount("the dose", e.dose, true)?;
    let taken_at = parse_datetime(&e.taken_at)?;
    let note = limits::text("the note", &e.note, limits::NOTE_MAX, false)?;
    let (medicine_id, status, _, old_deducted) = existing(tx, id)?;
    let deducted = if status == "taken" {
        stock::return_stock(tx, medicine_id, old_deducted)?;
        stock::take_stock(tx, medicine_id, dose)?
    } else {
        0.0
    };
    tx.execute(
        "UPDATE intakes SET taken_at = ?2, dose = ?3, note = ?4, deducted = ?5 WHERE id = ?1",
        params![id, taken_at, dose, note, deducted],
    )
    .map_err(db_err)?;
    Ok(())
}

/// Undo an intake: what a taken dose took goes back into the stock.
pub fn undo(tx: &Transaction, id: i64) -> Result<(), Error> {
    let (medicine_id, status, _, deducted) = existing(tx, id)?;
    tx.execute("DELETE FROM intakes WHERE id = ?1", [id])
        .map_err(db_err)?;
    if status == "taken" {
        stock::return_stock(tx, medicine_id, deducted)?;
    }
    Ok(())
}

/// Intakes whose local time starts with `prefix` (a day, `YYYY-MM-DD`) or
/// all of them newest first when `prefix` is empty, capped at `limit`.
pub fn list(conn: &Connection, prefix: &str, limit: i64) -> Result<Vec<Intake>, Error> {
    let mut stmt = conn
        .prepare(
            "SELECT i.id, i.medicine_id, m.name, i.taken_at, i.planned_for, i.status, i.dose, i.note
             FROM intakes i JOIN medicines m ON m.id = i.medicine_id
             WHERE m.deleted_at IS NULL AND substr(i.taken_at, 1, length(?1)) = ?1 ORDER BY i.taken_at DESC LIMIT ?2",
        )
        .map_err(db_err)?;
    let rows = stmt
        .query_map(params![prefix, limit], |r| {
            Ok(Intake {
                id: r.get(0)?,
                medicine_id: r.get(1)?,
                medicine_name: r.get(2)?,
                taken_at: r.get(3)?,
                planned_for: r.get(4)?,
                status: r.get(5)?,
                dose: r.get(6)?,
                note: r.get(7)?,
            })
        })
        .map_err(db_err)?;
    rows.collect::<Result<_, _>>().map_err(db_err)
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
