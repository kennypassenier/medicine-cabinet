//! Medicine forms (tablet, capsule, syrup, ...) and the unit each is counted in.

use chassis::Error;
use chassis::storage::rusqlite::{Connection, Transaction, params};
use serde::Serialize;

use super::{db_err, not_found};
use crate::domain::limits;

#[derive(Debug, Serialize)]
pub struct Form {
    pub id: i64,
    pub name: String,
    /// What stock of this form is counted in ("tablets", "ml").
    pub unit: String,
    pub medicines: i64,
}

pub fn list(conn: &Connection) -> Result<Vec<Form>, Error> {
    let mut stmt = conn
        .prepare(
            "SELECT f.id, f.name, f.unit, COUNT(m.id) FROM medicine_forms f
             LEFT JOIN medicines m ON m.form_id = f.id AND m.deleted_at IS NULL
             GROUP BY f.id ORDER BY f.name COLLATE NOCASE",
        )
        .map_err(db_err)?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Form {
                id: r.get(0)?,
                name: r.get(1)?,
                unit: r.get(2)?,
                medicines: r.get(3)?,
            })
        })
        .map_err(db_err)?;
    rows.collect::<Result<_, _>>().map_err(db_err)
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
    for medicine_id in medicine_ids {
        tx.execute(
            "UPDATE medicines SET form_id = ?2 WHERE id = ?1",
            params![medicine_id, id],
        )
        .map_err(db_err)?;
    }
    Ok(id)
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
    Ok(())
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
    tx.execute("DELETE FROM medicine_forms WHERE id = ?1", [id])
        .map_err(db_err)?;
    Ok(DeletedForm {
        name,
        unit,
        medicine_ids,
    })
}
