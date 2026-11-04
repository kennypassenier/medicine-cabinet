//! Stock movements: hand corrections and the amounts logged doses take.
//! Every amount here is whole hundredths (see [`crate::domain::quantity`]),
//! so repeated moves never drift.

use chassis::Error;
use chassis::storage::rusqlite::{Transaction, params};

use super::sync_log;
use super::{db_err, not_found};
use crate::domain::quantity;

/// Add (or, negative, remove) stock by hand. Stays within `0..=max`.
pub fn adjust_stock(tx: &Transaction, id: i64, delta: f64) -> Result<(), Error> {
    let delta = quantity::checked_signed("the stock change", delta)?;
    let changed = tx
        .execute(
            "UPDATE medicines SET stock_hundredths = MIN(?3, MAX(0, stock_hundredths + ?2))
             WHERE id = ?1 AND deleted_at IS NULL",
            params![id, delta, quantity::max_hundredths()],
        )
        .map_err(db_err)?;
    if changed == 0 {
        return Err(not_found("medicine", id));
    }
    sync_log::touch_stock(tx, id).map(|_| ())
}

/// Take up to `want` hundredths out of the stock for a logged dose; returns
/// what was really taken (never more than there is), so an undo can give it
/// back. The caller records the medicine's change (once per write, not per move).
pub fn take_stock(tx: &Transaction, id: i64, want: i64) -> Result<i64, Error> {
    let stock: i64 = tx
        .query_row(
            "SELECT stock_hundredths FROM medicines WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .map_err(|_| not_found("medicine", id))?;
    let taken = want.min(stock).max(0);
    tx.execute(
        "UPDATE medicines SET stock_hundredths = stock_hundredths - ?2 WHERE id = ?1",
        params![id, taken],
    )
    .map_err(db_err)?;
    Ok(taken)
}

/// Put back what [`take_stock`] took. The caller records the medicine's change.
pub fn return_stock(tx: &Transaction, id: i64, amount: i64) -> Result<(), Error> {
    tx.execute(
        "UPDATE medicines SET stock_hundredths = MIN(?3, stock_hundredths + ?2) WHERE id = ?1",
        params![id, amount, quantity::max_hundredths()],
    )
    .map_err(db_err)?;
    Ok(())
}

/// The stock of a medicine in hundredths (a missing medicine is a 404).
#[cfg(test)]
pub fn hundredths(tx: &Transaction, id: i64) -> Result<i64, Error> {
    tx.query_row(
        "SELECT stock_hundredths FROM medicines WHERE id = ?1",
        [id],
        |r| r.get(0),
    )
    .map_err(|_| not_found("medicine", id))
}
