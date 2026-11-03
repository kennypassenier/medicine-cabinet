//! Guarded writes: the `If-Match` check and the write happen in one
//! transaction, and the answer carries the revision the row ended at.

use chassis::storage::AppDb;
use chassis::storage::rusqlite::Transaction;
use chassis::{Conflict, Error, IfMatch};

use crate::store::sync_log::{self, Coll};

/// `Some(conflict)` when the request named a revision and the row has moved on.
/// A row that does not exist passes: the write itself answers 404.
fn stale(
    tx: &Transaction,
    if_match: IfMatch,
    coll: Coll,
    id: i64,
) -> Result<Option<Conflict>, Error> {
    Ok(match sync_log::current_rev(tx, coll, id)? {
        Some(current) => if_match.check(current).err(),
        None => None,
    })
}

/// The revision a write left behind: the row's own, or the last logged one
/// when the row is gone (a delete).
fn rev_after(tx: &Transaction, coll: Coll, id: i64) -> Result<u64, Error> {
    if let Some(rev) = sync_log::current_rev(tx, coll, id)? {
        return Ok(rev);
    }
    sync_log::logged_rev(tx, coll, &id.to_string())?.ok_or_else(|| {
        Error::internal(
            format!("{} {id} left no revision behind", coll.name()),
            "report this: a write on a synced row must record its change",
        )
    })
}

/// Run `write` on item `id` of `coll` unless `If-Match` is stale. Returns the
/// write's value with the new revision, or the 409 to send.
pub async fn guarded<T: Send + 'static>(
    db: &AppDb,
    if_match: IfMatch,
    coll: Coll,
    id: i64,
    write: impl FnOnce(&Transaction) -> Result<T, Error> + Send + 'static,
) -> Result<Result<(u64, T), Conflict>, Error> {
    db.write(move |tx| {
        if let Some(conflict) = stale(tx, if_match, coll, id)? {
            return Ok(Err(conflict));
        }
        let value = write(tx)?;
        Ok(Ok((rev_after(tx, coll, id)?, value)))
    })
    .await
}
