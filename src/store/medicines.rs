//! Medicines: the rows, plus their symptoms and schedule slots.

use chassis::Error;
use chassis::storage::rusqlite::{Transaction, params};

use super::refs::Keys;
use super::sync_log::{self, Coll};
use super::{db_err, not_found, symptoms};
use crate::domain::medicine::MedicineInput;
use crate::domain::quantity;
use crate::schema::UID_SQL;

pub use super::medicine_view::{Medicine, get, list};

/// A form id the request names must exist (a clear 404, not a database error).
fn ensure_form(tx: &Transaction, form_id: Option<i64>) -> Result<(), Error> {
    let Some(id) = form_id else { return Ok(()) };
    tx.query_row("SELECT 1 FROM medicine_forms WHERE id = ?1", [id], |_| {
        Ok(())
    })
    .map_err(|_| not_found("form", id))
}

/// Insert under a server-made `uid`.
pub fn insert(tx: &Transaction, m: &MedicineInput) -> Result<i64, Error> {
    insert_keyed(tx, m, &Keys::default())
}

/// Insert under the `uid` the browser made (the server makes one when
/// `None`), remembering the `Idempotency-Key` the request carried.
pub fn insert_keyed(tx: &Transaction, m: &MedicineInput, keys: &Keys) -> Result<i64, Error> {
    ensure_form(tx, m.form_id)?;
    let sql = format!(
        "INSERT INTO medicines (name, purpose, form_id, strength, notes, active, as_needed,
            min_gap_minutes, max_per_day, stock_hundredths, stock_unit,
            low_stock_at_hundredths, expires_on, opened_on, shelf_life_days, course_ends_on,
            uid, idempotency_key)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,
                 COALESCE(?17, {UID_SQL}), ?18)"
    );
    tx.execute(
        &sql,
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
            quantity::to_hundredths(m.stock),
            m.stock_unit,
            m.low_stock_at.map(quantity::to_hundredths),
            m.expires_on,
            m.opened_on,
            m.shelf_life_days,
            m.course_ends_on,
            keys.uid,
            keys.idempotency
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
                active=?7, as_needed=?8, min_gap_minutes=?9, max_per_day=?10,
                stock_hundredths=?11, stock_unit=?12, low_stock_at_hundredths=?13, expires_on=?14, opened_on=?15,
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
                quantity::to_hundredths(m.stock),
                m.stock_unit,
                m.low_stock_at.map(quantity::to_hundredths),
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
        // The logged doses carry a snapshot of the name; keep it current.
        tx.execute(
            "UPDATE intakes SET medicine_name = ?2 WHERE medicine_id = ?1",
            params![id, m.name],
        )
        .map_err(db_err)?;
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

/// Remove what has been deleted for longer than the restore window. The
/// medicine's logged doses stay: their `medicine_id` becomes NULL (the schema's
/// `ON DELETE SET NULL`) and they keep the name stored on them. They were
/// hidden from the sync feed while the medicine was soft-deleted, so they are
/// recorded again as upserts (never as deletes) now that they show for good.
pub fn purge_deleted(tx: &Transaction) -> Result<usize, Error> {
    let window = format!("-{PURGE_AFTER_DAYS} days");
    let mut orphaned = Vec::new();
    for id in expired_ids(tx, &window)? {
        orphaned.extend(sync_log::ids(
            tx,
            "SELECT id FROM intakes WHERE medicine_id = ?1",
            id,
        )?);
    }
    let purged = tx
        .execute(
            "DELETE FROM medicines WHERE deleted_at IS NOT NULL AND deleted_at < datetime('now', ?1)",
            [window],
        )
        .map_err(db_err)?;
    sync_log::touch_all(tx, Coll::Intakes, &orphaned)?;
    Ok(purged)
}

fn expired_ids(tx: &Transaction, window: &str) -> Result<Vec<i64>, Error> {
    let mut stmt = tx
        .prepare(
            "SELECT id FROM medicines WHERE deleted_at IS NOT NULL AND deleted_at < datetime('now', ?1)",
        )
        .map_err(db_err)?;
    let rows = stmt
        .query_map([window], |r| r.get::<_, i64>(0))
        .map_err(db_err)?;
    rows.collect::<Result<_, _>>().map_err(db_err)
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
