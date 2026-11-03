//! Schedule slots of a medicine.

use chassis::Error;
use chassis::storage::rusqlite::{Connection, Transaction, params};

use super::db_err;
use super::sync_log::{self, Coll};
use crate::domain::schedule::SlotInput;

pub fn replace(tx: &Transaction, medicine_id: i64, slots: &[SlotInput]) -> Result<(), Error> {
    tx.execute(
        "DELETE FROM schedule_slots WHERE medicine_id = ?1",
        [medicine_id],
    )
    .map_err(db_err)?;
    for s in slots {
        tx.execute(
            "INSERT INTO schedule_slots (medicine_id, time_of_day, days, dose)
             VALUES (?1, ?2, ?3, ?4)",
            params![medicine_id, s.time, s.days, s.dose],
        )
        .map_err(db_err)?;
    }
    // The slots are part of the medicine's body.
    sync_log::touch(tx, Coll::Medicines, medicine_id).map(|_| ())
}

pub fn for_medicine(conn: &Connection, medicine_id: i64) -> Result<Vec<SlotInput>, Error> {
    let mut stmt = conn
        .prepare(
            "SELECT time_of_day, days, dose FROM schedule_slots
             WHERE medicine_id = ?1 ORDER BY time_of_day",
        )
        .map_err(db_err)?;
    let rows = stmt
        .query_map([medicine_id], |r| {
            Ok(SlotInput {
                time: r.get(0)?,
                days: r.get(1)?,
                dose: r.get(2)?,
            })
        })
        .map_err(db_err)?;
    rows.collect::<Result<_, _>>().map_err(db_err)
}
