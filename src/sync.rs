//! The offline feed: which collections the browser's local database keeps and
//! how the current body of an item is read. Bodies are the very JSON the
//! `GET /api/mc/...` routes return, so a page can use either.

use std::collections::HashMap;

use chassis::Error;
use chassis::storage::rusqlite::Connection;
use chassis::storage::sync::{SyncCollection, SyncLoad};
use chrono_tz::Tz;
use serde::Serialize;
use serde_json::Value;

use crate::clock;
use crate::store::sync_log::Coll;
use crate::store::{categories, forms, intakes, medicine_view, refs, symptoms};

/// The five collections, with the routes the page's write queue replays to.
pub fn collections() -> Vec<SyncCollection> {
    vec![
        collection(Coll::Medicines, "/api/mc/medicines"),
        collection(Coll::Intakes, "/api/mc/intakes"),
        collection(Coll::Symptoms, "/api/mc/symptoms"),
        collection(Coll::Categories, "/api/mc/categories"),
        collection(Coll::Forms, "/api/mc/forms"),
    ]
}

/// An intake body carries its own `uid`; the other four carry their public
/// identifier as `id`. Both are the key the device stores the item under.
fn key_field(coll: Coll) -> &'static str {
    match coll {
        Coll::Intakes => "uid",
        _ => "id",
    }
}

fn collection(coll: Coll, list_route: &str) -> SyncCollection {
    SyncCollection::new(coll.name())
        .key(key_field(coll))
        .rev("rev")
        .shared()
        .routes(list_route, &format!("{list_route}/:id"))
}

/// Reads the current body of synced items. Medicines carry judgements that
/// depend on the date (freshness, course over); they are computed for today
/// in the configured zone.
pub struct Loader {
    zone: Tz,
}

impl Loader {
    pub fn new(zone: Tz) -> Self {
        Loader { zone }
    }
}

impl SyncLoad for Loader {
    fn load(
        &self,
        conn: &Connection,
        collection: &str,
        keys: &[String],
    ) -> Result<HashMap<String, Value>, Error> {
        let mut out = HashMap::new();
        for key in keys {
            if let Some(body) = self.body(conn, collection, key)? {
                out.insert(key.clone(), body);
            }
        }
        Ok(out)
    }
}

impl Loader {
    fn body(&self, conn: &Connection, collection: &str, key: &str) -> Result<Option<Value>, Error> {
        match collection {
            "medicines" => match refs::lookup(conn, Coll::Medicines, key)? {
                Some(id) => {
                    let today = clock::now_in(self.zone).date;
                    json(medicine_view::find(conn, id, today)?)
                }
                None => Ok(None),
            },
            "intakes" => json(intakes::get_by_uid(conn, key)?),
            "symptoms" => json_by_uid(conn, Coll::Symptoms, key, symptoms::get),
            "categories" => json_by_uid(conn, Coll::Categories, key, categories::get),
            "forms" => json_by_uid(conn, Coll::Forms, key, forms::get),
            other => Err(Error::internal(
                format!("the sync feed asked for an unknown collection `{other}`"),
                "report this: sync collections and the loader are out of step",
            )),
        }
    }
}

/// The body of the item `key` (a uid) of a collection, `None` when it is gone.
fn json_by_uid<T: Serialize>(
    conn: &Connection,
    coll: Coll,
    key: &str,
    get: impl Fn(&Connection, i64) -> Result<Option<T>, Error>,
) -> Result<Option<Value>, Error> {
    match refs::lookup(conn, coll, key)? {
        Some(id) => json(get(conn, id)?),
        None => Ok(None),
    }
}

fn json(item: Option<impl Serialize>) -> Result<Option<Value>, Error> {
    item.map(|i| {
        serde_json::to_value(i).map_err(|e| {
            Error::internal(
                format!("a synced item could not be written as JSON: {e}"),
                "report this: the item type is not serializable",
            )
        })
    })
    .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_collection_is_declared_with_its_routes() {
        let names: Vec<String> = collections().iter().map(|c| c.name().to_string()).collect();
        let expected: Vec<&str> = crate::store::sync_log::ALL
            .iter()
            .map(|c| c.name())
            .collect();
        assert_eq!(names, expected);
        assert!(collections().iter().all(|c| c.is_shared()));
    }

    #[test]
    fn every_collection_is_keyed_by_a_uid() {
        let keys: Vec<String> = collections()
            .iter()
            .map(|c| c.key_field().to_string())
            .collect();
        assert_eq!(keys, ["id", "uid", "id", "id", "id"]);
    }
}
