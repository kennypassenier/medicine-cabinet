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
            format!("The category '{name}' already exists"),
            "Pick another name, or edit the existing one.",
        );
    }
    db_err(e)
}

/// Create a category. `symptom_ids` files those symptoms under it, which is how an
/// undo of a delete brings the old grouping back.
pub fn create(tx: &Transaction, name: &str, symptom_ids: &[i64]) -> Result<i64, Error> {
    let name = clean_name(name)?;
    tx.execute("INSERT INTO symptom_categories (name) VALUES (?1)", [&name])
        .map_err(|e| name_taken(e, &name))?;
    let id = tx.last_insert_rowid();
    for symptom_id in symptom_ids {
        tx.execute(
            "UPDATE symptoms SET category_id = ?2 WHERE id = ?1",
            params![symptom_id, id],
        )
        .map_err(db_err)?;
    }
    Ok(id)
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

/// What a delete removed, so an undo can put it back.
#[derive(Debug, Serialize)]
pub struct DeletedCategory {
    pub name: String,
    pub symptom_ids: Vec<i64>,
}

/// Its symptoms stay; they just lose the category. The result lists them.
pub fn delete(tx: &Transaction, id: i64) -> Result<DeletedCategory, Error> {
    let name: String = tx
        .query_row(
            "SELECT name FROM symptom_categories WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .map_err(|_| not_found("category", id))?;
    let mut stmt = tx
        .prepare("SELECT id FROM symptoms WHERE category_id = ?1")
        .map_err(db_err)?;
    let symptom_ids = stmt
        .query_map([id], |r| r.get::<_, i64>(0))
        .map_err(db_err)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(db_err)?;
    drop(stmt);
    tx.execute("DELETE FROM symptom_categories WHERE id = ?1", [id])
        .map_err(db_err)?;
    Ok(DeletedCategory { name, symptom_ids })
}
