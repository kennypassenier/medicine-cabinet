//! Symptom names and their links to medicines.

use chassis::Error;
use chassis::storage::rusqlite::{Connection, Transaction, params};
use serde::Serialize;

use super::db_err;

/// Replace the symptom links of one medicine with exactly `names`.
pub fn set_for_medicine(tx: &Transaction, medicine_id: i64, names: &[String]) -> Result<(), Error> {
    tx.execute(
        "DELETE FROM medicine_symptoms WHERE medicine_id = ?1",
        [medicine_id],
    )
    .map_err(db_err)?;
    for name in names {
        tx.execute("INSERT OR IGNORE INTO symptoms (name) VALUES (?1)", [name])
            .map_err(db_err)?;
        tx.execute(
            "INSERT INTO medicine_symptoms (medicine_id, symptom_id)
             SELECT ?1, id FROM symptoms WHERE name = ?2",
            params![medicine_id, name],
        )
        .map_err(db_err)?;
    }
    Ok(())
}

pub fn for_medicine(conn: &Connection, medicine_id: i64) -> Result<Vec<String>, Error> {
    let mut stmt = conn
        .prepare(
            "SELECT s.name FROM symptoms s
             JOIN medicine_symptoms ms ON ms.symptom_id = s.id
             WHERE ms.medicine_id = ?1 ORDER BY s.name",
        )
        .map_err(db_err)?;
    let rows = stmt
        .query_map([medicine_id], |r| r.get::<_, String>(0))
        .map_err(db_err)?;
    rows.collect::<Result<_, _>>().map_err(db_err)
}

/// A symptom with its category and how many medicines treat it.
#[derive(Debug, Serialize)]
pub struct Symptom {
    pub id: i64,
    pub name: String,
    pub category_id: Option<i64>,
    pub category: Option<String>,
    pub medicines: i64,
}

pub fn list(conn: &Connection) -> Result<Vec<Symptom>, Error> {
    let mut stmt = conn
        .prepare(
            "SELECT s.id, s.name, s.category_id, c.name, COUNT(ms.medicine_id)
             FROM symptoms s
             LEFT JOIN symptom_categories c ON c.id = s.category_id
             LEFT JOIN medicine_symptoms ms ON ms.symptom_id = s.id
             GROUP BY s.id ORDER BY s.name",
        )
        .map_err(db_err)?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Symptom {
                id: r.get(0)?,
                name: r.get(1)?,
                category_id: r.get(2)?,
                category: r.get(3)?,
                medicines: r.get(4)?,
            })
        })
        .map_err(db_err)?;
    rows.collect::<Result<_, _>>().map_err(db_err)
}

fn clean_name(name: &str) -> Result<String, Error> {
    let name = name.trim().to_lowercase();
    if name.is_empty() {
        return Err(Error::invalid(
            "a symptom needs a name",
            "fill in the name field",
        ));
    }
    Ok(name)
}

fn name_taken(e: chassis::storage::rusqlite::Error, name: &str) -> Error {
    let text = e.to_string();
    if text.contains("UNIQUE") {
        return Error::invalid(
            format!("'{name}' already exists"),
            "pick another name, or edit the existing one",
        );
    }
    db_err(e)
}

pub fn create(tx: &Transaction, name: &str, category_id: Option<i64>) -> Result<i64, Error> {
    let name = clean_name(name)?;
    tx.execute(
        "INSERT INTO symptoms (name, category_id) VALUES (?1, ?2)",
        params![name, category_id],
    )
    .map_err(|e| name_taken(e, &name))?;
    Ok(tx.last_insert_rowid())
}

pub fn update(
    tx: &Transaction,
    id: i64,
    name: &str,
    category_id: Option<i64>,
) -> Result<(), Error> {
    let name = clean_name(name)?;
    let changed = tx
        .execute(
            "UPDATE symptoms SET name = ?2, category_id = ?3 WHERE id = ?1",
            params![id, name, category_id],
        )
        .map_err(|e| name_taken(e, &name))?;
    if changed == 0 {
        return Err(super::not_found("symptom", id));
    }
    Ok(())
}

/// Deleting a symptom unlinks it from every medicine.
pub fn delete(tx: &Transaction, id: i64) -> Result<(), Error> {
    let changed = tx
        .execute("DELETE FROM symptoms WHERE id = ?1", [id])
        .map_err(db_err)?;
    if changed == 0 {
        return Err(super::not_found("symptom", id));
    }
    Ok(())
}
