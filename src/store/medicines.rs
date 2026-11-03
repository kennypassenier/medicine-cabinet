//! Medicines: the rows, plus their symptoms and schedule slots.

use chassis::Error;
use chassis::storage::rusqlite::{Transaction, params};

use super::sync_log::{self, Coll};
use super::{db_err, not_found, symptoms};
use crate::domain::medicine::MedicineInput;

pub use super::medicine_view::{Medicine, get, list};

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
    sync_log::created(tx, Coll::Medicines, id)?;
    sync_log::touch_opt(tx, Coll::Forms, m.form_id)?;
    Ok(id)
}

pub fn update(tx: &Transaction, id: i64, m: &MedicineInput) -> Result<(), Error> {
    ensure_form(tx, m.form_id)?;
    let before = tx
        .query_row(
            "SELECT name, form_id FROM medicines WHERE id = ?1 AND deleted_at IS NULL",
            [id],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, Option<i64>>(1)?)),
        )
        .map_err(|_| not_found("medicine", id))?;
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
    symptoms::set_for_medicine(tx, id, &m.symptoms)?;
    record_update(tx, id, m, before)
}

/// What an update changed beyond the medicine itself: intakes show its name,
/// forms count their medicines.
fn record_update(
    tx: &Transaction,
    id: i64,
    m: &MedicineInput,
    (old_name, old_form): (String, Option<i64>),
) -> Result<(), Error> {
    sync_log::touch(tx, Coll::Medicines, id)?;
    if old_name != m.name {
        sync_log::touch_intakes_of(tx, id)?;
    }
    if old_form != m.form_id {
        sync_log::touch_opt(tx, Coll::Forms, old_form)?;
        sync_log::touch_opt(tx, Coll::Forms, m.form_id)?;
    }
    Ok(())
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
    touch_with_dependents(tx, id)
}

/// A medicine that is deleted or restored changes what its intakes, symptoms
/// and form show (a deleted medicine hides them and stops counting).
fn touch_with_dependents(tx: &Transaction, id: i64) -> Result<(), Error> {
    sync_log::touch(tx, Coll::Medicines, id)?;
    sync_log::touch_intakes_of(tx, id)?;
    let symptom_ids = sync_log::ids(
        tx,
        "SELECT symptom_id FROM medicine_symptoms WHERE medicine_id = ?1",
        id,
    )?;
    sync_log::touch_all(tx, Coll::Symptoms, &symptom_ids)?;
    let forms = sync_log::ids(
        tx,
        "SELECT form_id FROM medicines WHERE id = ?1 AND form_id IS NOT NULL",
        id,
    )?;
    sync_log::touch_all(tx, Coll::Forms, &forms)
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
    touch_with_dependents(tx, id)
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
    sync_log::touch(tx, Coll::Medicines, id).map(|_| ())
}
