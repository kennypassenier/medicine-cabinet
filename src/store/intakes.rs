//! The intake log: what was taken or skipped, and when.

use chassis::Error;
use chassis::storage::rusqlite::{Transaction, params};
use serde::Deserialize;

use super::sync_log::{self, Coll};
use super::{db_err, not_found, stock};
use crate::domain::date::parse_datetime;
use crate::domain::limits;
use crate::domain::quantity;
use crate::schema::UID_SQL;

pub use super::intake_queries::{
    Intake, find_held, get_by_uid, list, recent_taken_minutes_ago, resolve,
};

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

/// The columns every edit or undo needs.
struct Existing {
    /// `None` once the medicine was purged: such an intake moves no stock.
    medicine_id: Option<i64>,
    status: String,
    /// What the dose took out of the stock, in hundredths.
    deducted: i64,
}

fn existing(tx: &Transaction, id: i64) -> Result<Existing, Error> {
    tx.query_row(
        "SELECT medicine_id, status, COALESCE(deducted_hundredths, dose_hundredths)
         FROM intakes WHERE id = ?1",
        [id],
        |r| {
            Ok(Existing {
                medicine_id: r.get(0)?,
                status: r.get(1)?,
                deducted: r.get(2)?,
            })
        },
    )
    .map_err(|e| match e {
        chassis::storage::rusqlite::Error::QueryReturnedNoRows => not_found("intake", id),
        other => db_err(other),
    })
}

/// What a recorded intake is called, for the answer to the request.
#[derive(Debug, Clone, PartialEq)]
pub struct Recorded {
    pub id: i64,
    pub uid: String,
    pub rev: u64,
}

/// The validated parts of an [`IntakeInput`].
struct Checked {
    taken_at: String,
    planned_for: Option<String>,
    note: String,
    /// The dose in hundredths.
    dose: i64,
    /// The medicine's name, stored on the intake so the history outlives it.
    medicine_name: String,
}

fn check(tx: &Transaction, i: &IntakeInput) -> Result<Checked, Error> {
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
    let dose = quantity::checked("the dose", i.dose.unwrap_or(1.0), true)?;
    // A soft-deleted medicine is gone for logging purposes; a real database
    // failure must surface rather than read as "no such medicine".
    let medicine_name = match tx.query_row(
        "SELECT name FROM medicines WHERE id = ?1 AND deleted_at IS NULL",
        [i.medicine_id],
        |r| r.get::<_, String>(0),
    ) {
        Ok(name) => name,
        Err(chassis::storage::rusqlite::Error::QueryReturnedNoRows) => {
            return Err(not_found("medicine", i.medicine_id));
        }
        Err(e) => return Err(db_err(e)),
    };
    if let Some(slot) = &planned_for {
        refuse_if_answered(tx, i.medicine_id, slot)?;
    }
    Ok(Checked {
        taken_at,
        planned_for,
        note,
        dose,
        medicine_name,
    })
}

/// Record an intake; a taken dose also takes from the stock (never below zero).
#[cfg(test)]
pub fn record(tx: &Transaction, i: &IntakeInput) -> Result<i64, Error> {
    record_keyed(tx, i, None, None).map(|r| r.id)
}

/// [`record`] under a `uid` the browser made (the server makes one when
/// `None`), remembering the `Idempotency-Key` the request carried.
pub fn record_keyed(
    tx: &Transaction,
    i: &IntakeInput,
    uid: Option<&str>,
    idempotency_key: Option<&str>,
) -> Result<Recorded, Error> {
    let c = check(tx, i)?;
    let taken = i.status == "taken";
    let deducted = if taken {
        stock::take_stock(tx, i.medicine_id, c.dose)?
    } else {
        0
    };
    let sql = format!(
        "INSERT INTO intakes (medicine_id, medicine_name, taken_at, planned_for, status,
                              dose_hundredths, note, deducted_hundredths, uid, idempotency_key)
         VALUES (?1, ?10, ?2, ?3, ?4, ?5, ?6, ?7, COALESCE(?8, {UID_SQL}), ?9)"
    );
    tx.execute(
        &sql,
        params![
            i.medicine_id,
            c.taken_at,
            c.planned_for,
            i.status,
            c.dose,
            c.note,
            deducted,
            uid,
            idempotency_key,
            c.medicine_name
        ],
    )
    .map_err(db_err)?;
    let id = tx.last_insert_rowid();
    sync_log::created(tx, Coll::Intakes, id)?;
    if taken {
        sync_log::touch(tx, Coll::Medicines, i.medicine_id)?;
    }
    held_by_id(tx, id)
}

fn held_by_id(tx: &Transaction, id: i64) -> Result<Recorded, Error> {
    tx.query_row("SELECT uid, rev FROM intakes WHERE id = ?1", [id], |r| {
        Ok(Recorded {
            id,
            uid: r.get(0)?,
            rev: sync_log::read_rev(r, 1)?,
        })
    })
    .map_err(db_err)
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

/// Correct the time, dose or note of an intake; a changed dose moves the
/// stock. An intake whose medicine was purged moves none.
pub fn edit(tx: &Transaction, id: i64, e: &IntakeEdit) -> Result<(), Error> {
    let dose = quantity::checked("the dose", e.dose, true)?;
    let taken_at = parse_datetime(&e.taken_at)?;
    let note = limits::text("the note", &e.note, limits::NOTE_MAX, false)?;
    let old = existing(tx, id)?;
    let taken = old.status == "taken";
    let deducted = match old.medicine_id {
        Some(medicine_id) if taken => {
            stock::return_stock(tx, medicine_id, old.deducted)?;
            stock::take_stock(tx, medicine_id, dose)?
        }
        // Nothing to move: skipped, or the medicine is gone.
        _ => old.deducted,
    };
    tx.execute(
        "UPDATE intakes SET taken_at = ?2, dose_hundredths = ?3, note = ?4,
                            deducted_hundredths = ?5 WHERE id = ?1",
        params![id, taken_at, dose, note, deducted],
    )
    .map_err(db_err)?;
    sync_log::touch(tx, Coll::Intakes, id)?;
    if let (true, Some(medicine_id)) = (taken, old.medicine_id) {
        sync_log::touch(tx, Coll::Medicines, medicine_id)?;
    }
    Ok(())
}

/// Undo an intake: what a taken dose took goes back into the stock (nothing
/// goes back when its medicine was purged).
pub fn undo(tx: &Transaction, id: i64) -> Result<(), Error> {
    let old = existing(tx, id)?;
    sync_log::tombstone(tx, Coll::Intakes, id)?;
    tx.execute("DELETE FROM intakes WHERE id = ?1", [id])
        .map_err(db_err)?;
    if let (true, Some(medicine_id)) = (old.status == "taken", old.medicine_id) {
        stock::return_stock(tx, medicine_id, old.deducted)?;
        sync_log::touch(tx, Coll::Medicines, medicine_id)?;
    }
    Ok(())
}
