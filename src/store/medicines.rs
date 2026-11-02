//! Medicines: the rows, plus their symptoms and schedule slots.

use chassis::Error;
use chassis::storage::rusqlite::{Connection, Row, Transaction, params};
use serde::Serialize;

use super::{db_err, not_found, slots, symptoms};
use crate::domain::date::Date;
use crate::domain::medicine::{
    Freshness, MedicineInput, StockLevel, effective_expiry, freshness, stock_level,
};
use crate::domain::schedule::SlotInput;

/// A medicine as the API returns it, with the judgements the UI shows.
#[derive(Debug, Serialize)]
pub struct Medicine {
    pub id: i64,
    pub name: String,
    pub purpose: String,
    pub form: String,
    pub form_id: Option<i64>,
    pub strength: String,
    pub notes: String,
    pub active: bool,
    pub as_needed: bool,
    pub min_gap_minutes: Option<u32>,
    pub max_per_day: Option<u32>,
    pub stock: f64,
    pub stock_unit: String,
    pub low_stock_at: Option<f64>,
    pub expires_on: Option<String>,
    pub opened_on: Option<String>,
    pub shelf_life_days: Option<u32>,
    pub course_ends_on: Option<String>,
    pub symptoms: Vec<String>,
    pub slots: Vec<SlotInput>,
    /// The date the medicine stops being good (printed or after opening).
    pub effective_expiry: Option<String>,
    pub freshness: Freshness,
    pub stock_level: StockLevel,
    /// The course has ended: the schedule no longer applies.
    pub course_over: bool,
}

const COLUMNS: &str = "id, name, purpose,
    COALESCE((SELECT name FROM medicine_forms f WHERE f.id = medicines.form_id), ''), strength, notes, active, as_needed,
    min_gap_minutes, max_per_day, stock, stock_unit, low_stock_at,
    expires_on, opened_on, shelf_life_days, course_ends_on, form_id";

fn read_row(r: &Row, today: Date) -> Result<Medicine, chassis::storage::rusqlite::Error> {
    let expires_on: Option<String> = r.get(13)?;
    let opened_on: Option<String> = r.get(14)?;
    let shelf_life_days: Option<u32> = r.get(15)?;
    let course_ends_on: Option<String> = r.get(16)?;
    let stock: f64 = r.get(10)?;
    let low_stock_at: Option<f64> = r.get(12)?;
    let expiry = effective_expiry(expires_on.as_deref(), opened_on.as_deref(), shelf_life_days);
    let course_over = course_ends_on
        .as_deref()
        .and_then(|d| Date::parse(d).ok())
        .is_some_and(|end| end < today);
    Ok(Medicine {
        id: r.get(0)?,
        name: r.get(1)?,
        purpose: r.get(2)?,
        form: r.get(3)?,
        form_id: r.get(17)?,
        strength: r.get(4)?,
        notes: r.get(5)?,
        active: r.get::<_, i64>(6)? != 0,
        as_needed: r.get::<_, i64>(7)? != 0,
        min_gap_minutes: r.get(8)?,
        max_per_day: r.get(9)?,
        stock,
        stock_unit: r.get(11)?,
        low_stock_at,
        expires_on,
        opened_on,
        shelf_life_days,
        course_ends_on,
        symptoms: Vec::new(),
        slots: Vec::new(),
        effective_expiry: expiry.map(Date::iso),
        freshness: freshness(expiry, today),
        stock_level: stock_level(stock, low_stock_at),
        course_over,
    })
}

fn fill_children(conn: &Connection, mut m: Medicine) -> Result<Medicine, Error> {
    m.symptoms = symptoms::for_medicine(conn, m.id)?;
    m.slots = slots::for_medicine(conn, m.id)?;
    Ok(m)
}

pub fn list(conn: &Connection, today: Date) -> Result<Vec<Medicine>, Error> {
    let sql = format!(
        "SELECT {COLUMNS} FROM medicines WHERE deleted_at IS NULL ORDER BY active DESC, name COLLATE NOCASE"
    );
    let mut stmt = conn.prepare(&sql).map_err(db_err)?;
    let rows = stmt.query_map([], |r| read_row(r, today)).map_err(db_err)?;
    let mut out = Vec::new();
    for row in rows {
        out.push(fill_children(conn, row.map_err(db_err)?)?);
    }
    Ok(out)
}

pub fn get(conn: &Connection, id: i64, today: Date) -> Result<Medicine, Error> {
    let sql = format!("SELECT {COLUMNS} FROM medicines WHERE id = ?1 AND deleted_at IS NULL");
    let found = conn.query_row(&sql, [id], |r| read_row(r, today));
    match found {
        Ok(m) => fill_children(conn, m),
        Err(chassis::storage::rusqlite::Error::QueryReturnedNoRows) => {
            Err(not_found("medicine", id))
        }
        Err(e) => Err(db_err(e)),
    }
}

/// A form id the request names must exist (a clear 404, not a database error).
fn ensure_form(tx: &Transaction, form_id: Option<i64>) -> Result<(), Error> {
    let Some(id) = form_id else { return Ok(()) };
    tx.query_row("SELECT 1 FROM medicine_forms WHERE id = ?1", [id], |_| {
        Ok(())
    })
    .map_err(|_| not_found("form", id))
}

pub fn insert(tx: &Transaction, m: &MedicineInput) -> Result<i64, Error> {
    ensure_form(tx, m.form_id)?;
    tx.execute(
        "INSERT INTO medicines (name, purpose, form_id, strength, notes, active, as_needed,
            min_gap_minutes, max_per_day, stock, stock_unit, low_stock_at,
            expires_on, opened_on, shelf_life_days, course_ends_on)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16)",
        params![
            m.name,
            m.purpose,
            m.form_id,
            m.strength,
            m.notes,
            m.active,
            m.as_needed,
            m.min_gap_minutes,
            m.max_per_day,
            m.stock,
            m.stock_unit,
            m.low_stock_at,
            m.expires_on,
            m.opened_on,
            m.shelf_life_days,
            m.course_ends_on
        ],
    )
    .map_err(db_err)?;
    let id = tx.last_insert_rowid();
    symptoms::set_for_medicine(tx, id, &m.symptoms)?;
    Ok(id)
}

pub fn update(tx: &Transaction, id: i64, m: &MedicineInput) -> Result<(), Error> {
    ensure_form(tx, m.form_id)?;
    let changed = tx
        .execute(
            "UPDATE medicines SET name=?2, purpose=?3, form_id=?4, strength=?5, notes=?6,
                active=?7, as_needed=?8, min_gap_minutes=?9, max_per_day=?10, stock=?11,
                stock_unit=?12, low_stock_at=?13, expires_on=?14, opened_on=?15,
                shelf_life_days=?16, course_ends_on=?17 WHERE id=?1 AND deleted_at IS NULL",
            params![
                id,
                m.name,
                m.purpose,
                m.form_id,
                m.strength,
                m.notes,
                m.active,
                m.as_needed,
                m.min_gap_minutes,
                m.max_per_day,
                m.stock,
                m.stock_unit,
                m.low_stock_at,
                m.expires_on,
                m.opened_on,
                m.shelf_life_days,
                m.course_ends_on
            ],
        )
        .map_err(db_err)?;
    if changed == 0 {
        return Err(not_found("medicine", id));
    }
    symptoms::set_for_medicine(tx, id, &m.symptoms)
}

/// Soft delete: the row, its schedule and its history stay for [`PURGE_AFTER_DAYS`]
/// so `restore` can bring it back.
pub fn delete(tx: &Transaction, id: i64) -> Result<(), Error> {
    let changed = tx
        .execute(
            "UPDATE medicines SET deleted_at = datetime('now') WHERE id = ?1 AND deleted_at IS NULL",
            [id],
        )
        .map_err(db_err)?;
    if changed == 0 {
        return Err(not_found("medicine", id));
    }
    Ok(())
}

pub fn restore(tx: &Transaction, id: i64) -> Result<(), Error> {
    let changed = tx
        .execute(
            "UPDATE medicines SET deleted_at = NULL WHERE id = ?1 AND deleted_at IS NOT NULL",
            [id],
        )
        .map_err(db_err)?;
    if changed == 0 {
        return Err(not_found("deleted medicine", id));
    }
    Ok(())
}

/// How long a deleted medicine can still be restored.
pub const PURGE_AFTER_DAYS: i64 = 14;

/// Remove what has been deleted for longer than the restore window.
pub fn purge_deleted(tx: &Transaction) -> Result<usize, Error> {
    tx.execute(
        "DELETE FROM medicines WHERE deleted_at IS NOT NULL AND deleted_at < datetime('now', ?1)",
        [format!("-{PURGE_AFTER_DAYS} days")],
    )
    .map_err(db_err)
}

/// Switching a medicine off or on is the whole "pause this schedule" story:
/// the schedule is derived from `active`, so nothing else changes.
pub fn set_active(tx: &Transaction, id: i64, active: bool) -> Result<(), Error> {
    let changed = tx
        .execute(
            "UPDATE medicines SET active = ?2 WHERE id = ?1 AND deleted_at IS NULL",
            params![id, active],
        )
        .map_err(db_err)?;
    if changed == 0 {
        return Err(not_found("medicine", id));
    }
    Ok(())
}
