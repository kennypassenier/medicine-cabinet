//! Stock movements: hand corrections and the amounts logged doses take.

use chassis::Error;
use chassis::storage::rusqlite::{Transaction, params};

use super::sync_log::{self, Coll};
use super::{db_err, not_found};
use crate::domain::limits;

/// Add (or, negative, remove) stock by hand. Stays within `0..=AMOUNT_MAX`.
pub fn adjust_stock(tx: &Transaction, id: i64, delta: f64) -> Result<(), Error> {
    let delta = limits::signed_amount("the stock change", delta)?;
    let changed = tx
        .execute(
            "UPDATE medicines SET stock = MIN(?3, MAX(0, stock + ?2)) WHERE id = ?1 AND deleted_at IS NULL",
            params![id, delta, limits::AMOUNT_MAX],
        )
        .map_err(db_err)?;
    if changed == 0 {
        return Err(not_found("medicine", id));
    }
    sync_log::touch(tx, Coll::Medicines, id).map(|_| ())
}

/// Take up to `want` out of the stock for a logged dose; returns what was
/// really taken (never more than there is), so an undo can give it back.
/// The caller records the medicine's change (once per write, not per move).
pub fn take_stock(tx: &Transaction, id: i64, want: f64) -> Result<f64, Error> {
    let stock: f64 = tx
        .query_row("SELECT stock FROM medicines WHERE id = ?1", [id], |r| {
            r.get(0)
        })
        .map_err(|_| not_found("medicine", id))?;
    let taken = want.min(stock).max(0.0);
    tx.execute(
        "UPDATE medicines SET stock = MAX(0, stock - ?2) WHERE id = ?1",
        params![id, taken],
    )
    .map_err(db_err)?;
    Ok(taken)
}

/// Put back what [`take_stock`] took. The caller records the medicine's change.
pub fn return_stock(tx: &Transaction, id: i64, amount: f64) -> Result<(), Error> {
    tx.execute(
        "UPDATE medicines SET stock = MIN(?3, stock + ?2) WHERE id = ?1",
        params![id, amount, limits::AMOUNT_MAX],
    )
    .map_err(db_err)?;
    Ok(())
}
