//! Medicine forms (tablet, capsule, syrup, ...) and the unit each is counted in.

use chassis::Error;
use chassis::storage::rusqlite::{Connection, Transaction, params};
use serde::Serialize;

use super::sync_log::{self, Coll};
use super::{db_err, not_found};
use crate::domain::limits;

#[derive(Debug, Serialize)]
pub struct Form {
    pub id: i64,
    pub rev: u64,
    pub name: String,
    /// What stock of this form is counted in ("tablets", "ml").
    pub unit: String,
    pub medicines: i64,
}

const LIST_SQL: &str = "SELECT f.id, f.name, f.unit, COUNT(m.id), f.rev FROM medicine_forms f
     LEFT JOIN medicines m ON m.form_id = f.id AND m.deleted_at IS NULL
     WHERE (?1 IS NULL OR f.id = ?1)
     GROUP BY f.id ORDER BY f.name COLLATE NOCASE";

fn query(conn: &Connection, only: Option<i64>) -> Result<Vec<Form>, Error> {
    let mut stmt = conn.prepare(LIST_SQL).map_err(db_err)?;
    let rows = stmt
        .query_map([only], |r| {
            Ok(Form {
                id: r.get(0)?,
                name: r.get(1)?,
                unit: r.get(2)?,
                medicines: r.get(3)?,
                rev: sync_log::read_rev(r, 4)?,
            })
        })
        .map_err(db_err)?;
    rows.collect::<Result<_, _>>().map_err(db_err)
}

pub fn list(conn: &Connection) -> Result<Vec<Form>, Error> {
    query(conn, None)
}

/// One form as the list shows it, for the offline feed.
pub fn get(conn: &Connection, id: i64) -> Result<Option<Form>, Error> {
    Ok(query(conn, Some(id))?.pop())
}

fn clean(name: &str, unit: &str) -> Result<(String, String), Error> {
    let name = limits::text("the form name", name, limits::NAME_MAX, true)?;
    if name.is_empty() {
        return Err(Error::invalid(
            "a form needs a name",
            "fill in the name field",
        ));
    }
    Ok((
        name,
        limits::text("the unit", unit, limits::UNIT_MAX, true)?,
    ))
}

fn name_taken(e: chassis::storage::rusqlite::Error, name: &str) -> Error {
    if e.to_string().contains("UNIQUE") {
        return Error::invalid(
            format!("The form '{name}' already exists"),
            "Pick another name, or edit the existing one.",
        );
    }
    db_err(e)
}

/// Create a form. `medicine_ids` sets this form on those medicines, which is how an
/// undo of a delete brings them back.
pub fn create(
    tx: &Transaction,
    name: &str,
    unit: &str,
    medicine_ids: &[i64],
) -> Result<i64, Error> {
    limits::list_len("the medicine list", medicine_ids.len())?;
    let (name, unit) = clean(name, unit)?;
    tx.execute(
        "INSERT INTO medicine_forms (name, unit) VALUES (?1, ?2)",
        params![name, unit],
    )
    .map_err(|e| name_taken(e, &name))?;
    let id = tx.last_insert_rowid();
    sync_log::created(tx, Coll::Forms, id)?;
    set_on_medicines(tx, id, medicine_ids)?;
    Ok(id)
}

/// Set a form on medicines. Each medicine shows its form's name, and the
/// form it leaves counts one medicine fewer.
fn set_on_medicines(tx: &Transaction, form_id: i64, medicine_ids: &[i64]) -> Result<(), Error> {
    for medicine_id in medicine_ids {
        let old = sync_log::ids(
            tx,
            "SELECT form_id FROM medicines WHERE id = ?1 AND form_id IS NOT NULL",
            *medicine_id,
        )?;
        tx.execute(
            "UPDATE medicines SET form_id = ?2 WHERE id = ?1",
            params![medicine_id, form_id],
        )
        .map_err(db_err)?;
        sync_log::touch(tx, Coll::Medicines, *medicine_id)?;
        sync_log::touch_all(tx, Coll::Forms, &old)?;
    }
    sync_log::touch(tx, Coll::Forms, form_id).map(|_| ())
}

pub fn update(tx: &Transaction, id: i64, name: &str, unit: &str) -> Result<(), Error> {
    let (name, unit) = clean(name, unit)?;
    let changed = tx
        .execute(
            "UPDATE medicine_forms SET name = ?2, unit = ?3 WHERE id = ?1",
            params![id, name, unit],
        )
        .map_err(|e| name_taken(e, &name))?;
    if changed == 0 {
        return Err(not_found("form", id));
    }
    sync_log::touch(tx, Coll::Forms, id)?;
    // Medicines show the name of their form.
    let members = sync_log::ids(tx, "SELECT id FROM medicines WHERE form_id = ?1", id)?;
    sync_log::touch_all(tx, Coll::Medicines, &members)
}

/// What a delete removed, so an undo can put it back.
#[derive(Debug, Serialize)]
pub struct DeletedForm {
    pub name: String,
    pub unit: String,
    pub medicine_ids: Vec<i64>,
}

/// Medicines of this form keep existing; they just have no form any more.
pub fn delete(tx: &Transaction, id: i64) -> Result<DeletedForm, Error> {
    let (name, unit): (String, String) = tx
        .query_row(
            "SELECT name, unit FROM medicine_forms WHERE id = ?1",
            [id],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(|_| not_found("form", id))?;
    let mut stmt = tx
        .prepare("SELECT id FROM medicines WHERE form_id = ?1")
        .map_err(db_err)?;
    let medicine_ids = stmt
        .query_map([id], |r| r.get::<_, i64>(0))
        .map_err(db_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_err)?;
    drop(stmt);
    sync_log::tombstone(tx, Coll::Forms, id)?;
    tx.execute("DELETE FROM medicine_forms WHERE id = ?1", [id])
        .map_err(db_err)?;
    sync_log::touch_all(tx, Coll::Medicines, &medicine_ids)?;
    Ok(DeletedForm {
        name,
        unit,
        medicine_ids,
    })
}
