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
use crate::store::{categories, forms, intakes, medicine_view, symptoms};

/// The five collections, with the routes the page's write queue replays to.
pub fn collections() -> Vec<SyncCollection> {
    vec![
        collection(Coll::Medicines, "id", "/api/mc/medicines"),
        collection(Coll::Intakes, "uid", "/api/mc/intakes"),
        collection(Coll::Symptoms, "id", "/api/mc/symptoms"),
        collection(Coll::Categories, "id", "/api/mc/categories"),
        collection(Coll::Forms, "id", "/api/mc/forms"),
    ]
}

fn collection(coll: Coll, key: &str, list_route: &str) -> SyncCollection {
    SyncCollection::new(coll.name())
        .key(key)
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
            "medicines" => {
                let today = clock::now_in(self.zone).date;
                json(medicine_view::find(conn, numeric(key)?, today)?)
            }
            "intakes" => json(intakes::get_by_uid(conn, key)?),
            "symptoms" => json(symptoms::get(conn, numeric(key)?)?),
            "categories" => json(categories::get(conn, numeric(key)?)?),
            "forms" => json(forms::get(conn, numeric(key)?)?),
            other => Err(Error::internal(
                format!("the sync feed asked for an unknown collection `{other}`"),
                "report this: sync collections and the loader are out of step",
            )),
        }
    }
}

fn numeric(key: &str) -> Result<i64, Error> {
    key.parse().map_err(|_| {
        Error::internal(
            format!("the sync key `{key}` is not a number"),
            "report this: the change log holds a key this collection never writes",
        )
    })
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
    fn the_intake_collection_is_keyed_by_uid() {
        let intakes = &collections()[1];
        assert_eq!(intakes.key_field(), "uid");
    }

    #[test]
    fn a_non_numeric_key_is_an_error_not_a_silent_skip() {
        assert!(numeric("3f2b8c1e").is_err());
        assert_eq!(numeric("12").unwrap(), 12);
    }
}
