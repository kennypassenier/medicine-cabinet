//! Symptom categories ("pain", "digestion", ...).

use chassis::Error;
use chassis::storage::rusqlite::{Connection, Transaction, params};
use serde::Serialize;

use super::refs::{self, Keys};
use super::sync_log::{self, Coll};
use super::{db_err, not_found};
use crate::domain::limits;
use crate::schema::UID_SQL;

#[derive(Debug, Serialize)]
pub struct Category {
    /// The public identifier; the internal integer id is never read back.
    #[serde(rename = "id")]
    pub uid: String,
    pub rev: u64,
    pub name: String,
    pub symptoms: i64,
}

const LIST_SQL: &str = "SELECT c.id, c.name, COUNT(s.id), c.rev, c.uid FROM symptom_categories c
     LEFT JOIN symptoms s ON s.category_id = c.id
     WHERE (?1 IS NULL OR c.id = ?1)
     GROUP BY c.id ORDER BY c.name COLLATE NOCASE";

fn query(conn: &Connection, only: Option<i64>) -> Result<Vec<Category>, Error> {
    let mut stmt = conn.prepare(LIST_SQL).map_err(db_err)?;
    let rows = stmt
        .query_map([only], |r| {
            Ok(Category {
                name: r.get(1)?,
                symptoms: r.get(2)?,
                rev: sync_log::read_rev(r, 3)?,
                uid: r.get(4)?,
            })
        })
        .map_err(db_err)?;
    rows.collect::<Result<_, _>>().map_err(db_err)
}

pub fn list(conn: &Connection) -> Result<Vec<Category>, Error> {
    query(conn, None)
}

/// One category as the list shows it, for the offline feed.
pub fn get(conn: &Connection, id: i64) -> Result<Option<Category>, Error> {
    Ok(query(conn, Some(id))?.pop())
}

fn clean_name(name: &str) -> Result<String, Error> {
    let name = limits::text("the category name", name, limits::NAME_MAX, true)?;
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
#[cfg(any(test, debug_assertions))]
pub fn create(tx: &Transaction, name: &str, symptom_ids: &[i64]) -> Result<i64, Error> {
    create_keyed(tx, name, symptom_ids, &Keys::default())
}

/// [`create`] under the `uid` the browser made and the `Idempotency-Key` of the request.
pub fn create_keyed(
    tx: &Transaction,
    name: &str,
    symptom_ids: &[i64],
    keys: &Keys,
) -> Result<i64, Error> {
    limits::list_len("the symptom list", symptom_ids.len())?;
    let name = clean_name(name)?;
    let sql = format!(
        "INSERT INTO symptom_categories (name, uid, idempotency_key)
         VALUES (?1, COALESCE(?2, {UID_SQL}), ?3)"
    );
    tx.execute(&sql, params![name, keys.uid, keys.idempotency])
        .map_err(|e| name_taken(e, &name))?;
    let id = tx.last_insert_rowid();
    sync_log::created(tx, Coll::Categories, id)?;
    file_symptoms(tx, id, symptom_ids)?;
    Ok(id)
}

/// File symptoms under a category. Each symptom shows its category name, and
/// the category it leaves counts one symptom fewer.
fn file_symptoms(tx: &Transaction, category_id: i64, symptom_ids: &[i64]) -> Result<(), Error> {
    for symptom_id in symptom_ids {
        let old = sync_log::ids(
            tx,
            "SELECT category_id FROM symptoms WHERE id = ?1 AND category_id IS NOT NULL",
            *symptom_id,
        )?;
        tx.execute(
            "UPDATE symptoms SET category_id = ?2 WHERE id = ?1",
            params![symptom_id, category_id],
        )
        .map_err(db_err)?;
        sync_log::touch(tx, Coll::Symptoms, *symptom_id)?;
        sync_log::touch_all(tx, Coll::Categories, &old)?;
    }
    sync_log::touch(tx, Coll::Categories, category_id).map(|_| ())
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
    sync_log::touch(tx, Coll::Categories, id)?;
    // Symptoms show the name of their category.
    let members = sync_log::ids(tx, "SELECT id FROM symptoms WHERE category_id = ?1", id)?;
    sync_log::touch_all(tx, Coll::Symptoms, &members)
}

/// What a delete removed, so an undo can put it back.
#[derive(Debug, Serialize)]
pub struct DeletedCategory {
    pub name: String,
    /// The `uid`s of the symptoms it held.
    pub symptom_ids: Vec<String>,
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
    sync_log::tombstone(tx, Coll::Categories, id)?;
    tx.execute("DELETE FROM symptom_categories WHERE id = ?1", [id])
        .map_err(db_err)?;
    sync_log::touch_all(tx, Coll::Symptoms, &symptom_ids)?;
    Ok(DeletedCategory {
        name,
        symptom_ids: refs::uids_of(tx, Coll::Symptoms, &symptom_ids)?,
    })
}
