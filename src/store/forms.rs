//! Medicine forms (tablet, capsule, syrup, ...) and the unit each is counted in.

use chassis::Error;
use chassis::storage::rusqlite::{Connection, Transaction, params};
use serde::Serialize;

use super::{db_err, not_found};

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
             LEFT JOIN medicines m ON m.form_id = f.id
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
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(Error::invalid(
            "a form needs a name",
            "fill in the name field",
        ));
    }
    Ok((name, unit.trim().to_string()))
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

pub fn create(tx: &Transaction, name: &str, unit: &str) -> Result<i64, Error> {
    let (name, unit) = clean(name, unit)?;
    tx.execute(
        "INSERT INTO medicine_forms (name, unit) VALUES (?1, ?2)",
        params![name, unit],
    )
    .map_err(|e| name_taken(e, &name))?;
    Ok(tx.last_insert_rowid())
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

/// Medicines of this form keep existing; they just have no form any more.
pub fn delete(tx: &Transaction, id: i64) -> Result<(), Error> {
    let changed = tx
        .execute("DELETE FROM medicine_forms WHERE id = ?1", [id])
        .map_err(db_err)?;
    if changed == 0 {
        return Err(not_found("form", id));
    }
    Ok(())
}
