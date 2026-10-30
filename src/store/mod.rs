//! Persistence: SQL lives here and nowhere else.

pub mod categories;
pub mod forms;
pub mod intakes;
pub mod medicines;
pub mod slots;
pub mod symptoms;

use chassis::Error;
use chassis::storage::rusqlite;

/// Turn a sqlite failure into the kit's error, keeping the cause visible.
pub fn db_err(e: rusqlite::Error) -> Error {
    Error::dependency(
        format!("the database refused the request: {e}"),
        "check the Storage page for the database state; if it is full or locked, retry in a moment",
    )
}

pub fn not_found(what: &str, id: i64) -> Error {
    Error::new(
        chassis::Kind::NotFound,
        format!("{what} {id} does not exist"),
        "reload the list; it may have been deleted",
    )
}
