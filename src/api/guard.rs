//! Guarded writes: the `If-Match` check and the write happen in one
//! transaction, and the answer carries the revision the row ended at.

use chassis::storage::AppDb;
use chassis::storage::rusqlite::Transaction;
use chassis::{Conflict, Error, IfMatch};

use crate::store::refs;
use crate::store::sync_log::{self, Coll};

/// What a guarded write did.
#[derive(Debug)]
pub struct Done<T> {
    /// The internal id the reference resolved to (an intake's id is public).
    pub id: i64,
    /// The sync key: the public `uid`.
    pub key: String,
    /// The revision the row ended at.
    pub rev: u64,
    /// What the write returned.
    pub value: T,
}

/// `Some(conflict)` when the request named a revision and the row has moved on.
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
fn rev_after(tx: &Transaction, coll: Coll, id: i64, key: &str) -> Result<u64, Error> {
    if let Some(rev) = sync_log::current_rev(tx, coll, id)? {
        return Ok(rev);
    }
    sync_log::logged_rev(tx, coll, key)?.ok_or_else(|| {
        Error::internal(
            format!("{} {key} left no revision behind", coll.name()),
            "report this: a write on a synced row must record its change",
        )
    })
}

/// Resolve `reference` (the route's `{id}`: a uid, or an intake's number),
/// then run `write` on that row of `coll` unless `If-Match` is stale. Returns
/// what the write did, or the 409 to send. An unknown row answers 404, a
/// malformed uid 400.
pub async fn guarded<T: Send + 'static>(
    db: &AppDb,
    if_match: IfMatch,
    coll: Coll,
    reference: String,
    write: impl FnOnce(&Transaction, i64) -> Result<T, Error> + Send + 'static,
) -> Result<Result<Done<T>, Conflict>, Error> {
    db.write(move |tx| {
        let id = refs::path(tx, coll, &reference)?;
        let key = sync_log::key_of(tx, coll, id)?.unwrap_or_default();
        if let Some(conflict) = stale(tx, if_match, coll, id)? {
            return Ok(Err(conflict));
        }
        let value = write(tx, id)?;
        let rev = rev_after(tx, coll, id, &key)?;
        Ok(Ok(Done {
            id,
            key,
            rev,
            value,
        }))
    })
    .await
}
