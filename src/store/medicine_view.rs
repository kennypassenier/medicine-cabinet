//! Reading medicines: the row with its symptoms and schedule, plus the
//! judgements the UI shows.

use chassis::Error;
use chassis::storage::rusqlite::{Connection, Row};
use serde::Serialize;

use super::{db_err, not_found, slots, symptoms};
use crate::domain::date::Date;
use crate::domain::medicine::{Freshness, StockLevel, effective_expiry, freshness, stock_level};
use crate::domain::schedule::SlotInput;

/// A medicine as the API returns it, with the judgements the UI shows.
#[derive(Debug, Serialize)]
pub struct Medicine {
    pub id: i64,
    /// Revision for the offline feed; bumped by every write.
    pub rev: u64,
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
    expires_on, opened_on, shelf_life_days, course_ends_on, form_id, rev";

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
        rev: super::sync_log::read_rev(r, 18)?,
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

/// One live medicine, or `None` when it does not exist or is deleted.
pub fn find(conn: &Connection, id: i64, today: Date) -> Result<Option<Medicine>, Error> {
    let sql = format!("SELECT {COLUMNS} FROM medicines WHERE id = ?1 AND deleted_at IS NULL");
    match conn.query_row(&sql, [id], |r| read_row(r, today)) {
        Ok(m) => fill_children(conn, m).map(Some),
        Err(chassis::storage::rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(e) => Err(db_err(e)),
    }
}

pub fn get(conn: &Connection, id: i64, today: Date) -> Result<Medicine, Error> {
    find(conn, id, today)?.ok_or_else(|| not_found("medicine", id))
}
