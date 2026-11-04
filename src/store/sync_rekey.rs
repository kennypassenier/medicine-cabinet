//! One-time re-keying of the change log from integer ids to uids.
//!
//! Until migration V10 the medicines, symptoms, categories and forms
//! collections were keyed by the integer id. A device that synced then holds
//! records under those keys. Every live integer-keyed entry becomes a delete
//! (a tombstone the feed serves until retention drops it), so a device
//! removes its old copies, and every live row is then recorded under its uid.
//! A device that stayed away longer than the retention window gets the kit's
//! 410 and starts over. Edits it still had queued under an integer id are
//! refused by the routes (not a UUID) and land in its failed list.

use chassis::Error;
use chassis::storage::rusqlite::Transaction;
use chassis::storage::sync::SyncWrite;

use super::db_err;
use super::sync_log::{self, Coll};

/// The four collections whose key changed (intakes were always keyed by uid).
const REKEYED: [Coll; 4] = [
    Coll::Medicines,
    Coll::Symptoms,
    Coll::Categories,
    Coll::Forms,
];

pub fn rekey_to_uids(tx: &Transaction) -> Result<(), Error> {
    for coll in REKEYED {
        for (key, rev) in integer_keyed(tx, coll)? {
            tx.sync_delete(coll.name(), &key, rev)?;
        }
    }
    sync_log::backfill(tx).map(|_| ())
}

/// Live change-log entries whose key is a plain integer (the old scheme).
fn integer_keyed(tx: &Transaction, coll: Coll) -> Result<Vec<(String, u64)>, Error> {
    let mut stmt = tx
        .prepare(
            "SELECT key, rev FROM chassis_sync
             WHERE collection = ?1 AND deleted = 0 AND key NOT GLOB '*[^0-9]*'",
        )
        .map_err(db_err)?;
    let rows = stmt
        .query_map([coll.name()], |r| {
            Ok((r.get::<_, String>(0)?, sync_log::read_rev(r, 1)?))
        })
        .map_err(db_err)?;
    rows.collect::<Result<_, _>>().map_err(db_err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testkit::{logged, sync_db};

    #[test]
    fn integer_keys_become_tombstones_and_uids_are_recorded() {
        let mut conn = sync_db();
        conn.execute(
            "INSERT INTO symptom_categories (id, name, uid) VALUES (7, 'Head', 'a')",
            [],
        )
        .unwrap();
        let tx = conn.transaction().unwrap();
        tx.sync_upsert("categories", "7", 3).unwrap();
        tx.sync_upsert("categories", "12", 1).unwrap();
        rekey_to_uids(&tx).unwrap();
        assert_eq!(logged(&tx, "categories", "7"), Some((3, true)));
        assert_eq!(logged(&tx, "categories", "12"), Some((1, true)));
        assert_eq!(logged(&tx, "categories", "a"), Some((1, false)));
        // A second run changes nothing.
        rekey_to_uids(&tx).unwrap();
        assert_eq!(logged(&tx, "categories", "a"), Some((1, false)));
    }
}
