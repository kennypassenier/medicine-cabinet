//! The intake log: what was taken or skipped, and when.

use chassis::Error;
use chassis::storage::rusqlite::{Connection, Transaction, params};
use serde::{Deserialize, Serialize};

use super::{db_err, medicines, not_found};

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

/// Record an intake; a taken dose also takes from the stock.
pub fn record(tx: &Transaction, i: &IntakeInput) -> Result<i64, Error> {
    if i.status != "taken" && i.status != "skipped" {
        return Err(Error::invalid(
            format!("status '{}' is not known", i.status),
            "use 'taken' or 'skipped'",
        ));
    }
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
    let dose = i.dose.unwrap_or(1.0);
    tx.execute(
        "INSERT INTO intakes (medicine_id, taken_at, planned_for, status, dose, note)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            i.medicine_id,
            i.taken_at,
            i.planned_for,
            i.status,
            dose,
            i.note
        ],
    )
    .map_err(db_err)?;
    let id = tx.last_insert_rowid();
    if i.status == "taken" {
        medicines::adjust_stock(tx, i.medicine_id, -dose)?;
    }
    Ok(id)
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

/// The medicine, status and dose of one intake; a missing row is "not found",
/// any other database failure stays a database failure.
fn intake_row(tx: &Transaction, id: i64) -> Result<(i64, String, f64), Error> {
    tx.query_row(
        "SELECT medicine_id, status, dose FROM intakes WHERE id = ?1",
        [id],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
    .map_err(|e| match e {
        chassis::storage::rusqlite::Error::QueryReturnedNoRows => not_found("intake", id),
        other => db_err(other),
    })
}

/// Correct the time, dose or note of an intake; a changed dose moves the stock.
pub fn edit(tx: &Transaction, id: i64, e: &IntakeEdit) -> Result<(), Error> {
    if e.dose <= 0.0 {
        return Err(Error::invalid(
            "a dose must be more than zero",
            "enter the amount that was taken",
        ));
    }
    let (medicine_id, status, old_dose) = intake_row(tx, id)?;
    tx.execute(
        "UPDATE intakes SET taken_at = ?2, dose = ?3, note = ?4 WHERE id = ?1",
        params![id, e.taken_at, e.dose, e.note],
    )
    .map_err(db_err)?;
    if status == "taken" {
        medicines::adjust_stock(tx, medicine_id, old_dose - e.dose)?;
    }
    Ok(())
}

/// Undo an intake: a taken dose goes back into the stock.
pub fn undo(tx: &Transaction, id: i64) -> Result<(), Error> {
    let (medicine_id, status, dose) = intake_row(tx, id)?;
    tx.execute("DELETE FROM intakes WHERE id = ?1", [id])
        .map_err(db_err)?;
    if status == "taken" {
        medicines::adjust_stock(tx, medicine_id, dose)?;
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
    for row in rows {
        let text = row.map_err(db_err)?;
        let day = crate::domain::date::Date::parse(&text)?.to_days();
        let minutes = i64::from(crate::domain::date::parse_minutes(&text)?);
        let ago = (now_days - day) * 1440 + now_minutes - minutes;
        if ago >= 0 {
            out.push(ago);
        }
    }
    Ok(out)
}
