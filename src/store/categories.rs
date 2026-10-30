//! Symptom categories ("pain", "digestion", ...).

use chassis::Error;
use chassis::storage::rusqlite::{Connection, Transaction, params};
use serde::Serialize;

use super::{db_err, not_found};

#[derive(Debug, Serialize)]
pub struct Category {
    pub id: i64,
    pub name: String,
    pub symptoms: i64,
}

pub fn list(conn: &Connection) -> Result<Vec<Category>, Error> {
    let mut stmt = conn
        .prepare(
            "SELECT c.id, c.name, COUNT(s.id) FROM symptom_categories c
             LEFT JOIN symptoms s ON s.category_id = c.id
             GROUP BY c.id ORDER BY c.name COLLATE NOCASE",
        )
        .map_err(db_err)?;
    let rows = stmt
        .query_map([], |r| {
            Ok(Category {
                id: r.get(0)?,
                name: r.get(1)?,
                symptoms: r.get(2)?,
            })
        })
        .map_err(db_err)?;
    rows.collect::<Result<_, _>>().map_err(db_err)
}

fn clean_name(name: &str) -> Result<String, Error> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(Error::invalid(
            "a category needs a name",
            "fill in the name field",
        ));
    }
    Ok(name)
}

fn name_taken(e: chassis::storage::rusqlite::Error, name: &str) -> Error {
    if e.to_string().contains("UNIQUE") {
        return Error::invalid(
            format!("the category '{name}' already exists"),
            "pick another name, or edit the existing one",
        );
    }
    db_err(e)
}

pub fn create(tx: &Transaction, name: &str) -> Result<i64, Error> {
    let name = clean_name(name)?;
    tx.execute("INSERT INTO symptom_categories (name) VALUES (?1)", [&name])
        .map_err(|e| name_taken(e, &name))?;
    Ok(tx.last_insert_rowid())
}

pub fn rename(tx: &Transaction, id: i64, name: &str) -> Result<(), Error> {
    let name = clean_name(name)?;
    let changed = tx
        .execute(
            "UPDATE symptom_categories SET name = ?2 WHERE id = ?1",
            params![id, name],
        )
        .map_err(|e| name_taken(e, &name))?;
    if changed == 0 {
        return Err(not_found("category", id));
    }
    Ok(())
}

/// Its symptoms stay; they just lose the category.
pub fn delete(tx: &Transaction, id: i64) -> Result<(), Error> {
    let changed = tx
        .execute("DELETE FROM symptom_categories WHERE id = ?1", [id])
        .map_err(db_err)?;
    if changed == 0 {
        return Err(not_found("category", id));
    }
    Ok(())
}
