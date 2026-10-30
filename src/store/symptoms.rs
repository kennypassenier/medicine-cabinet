//! Symptom names and their links to medicines.

use chassis::Error;
use chassis::storage::rusqlite::{Connection, Transaction, params};

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
    // Names nobody links to any more are dropped, so suggestions stay clean.
    tx.execute(
        "DELETE FROM symptoms WHERE id NOT IN (SELECT symptom_id FROM medicine_symptoms)",
        [],
    )
    .map_err(db_err)?;
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

/// Every known symptom with how many medicines treat it (for suggestions).
pub fn all_with_counts(conn: &Connection) -> Result<Vec<(String, i64)>, Error> {
    let mut stmt = conn
        .prepare(
            "SELECT s.name, COUNT(ms.medicine_id) FROM symptoms s
             JOIN medicine_symptoms ms ON ms.symptom_id = s.id
             GROUP BY s.id ORDER BY s.name",
        )
        .map_err(db_err)?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
        .map_err(db_err)?;
    rows.collect::<Result<_, _>>().map_err(db_err)
}
