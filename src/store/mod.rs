//! Persistence: SQL lives here and nowhere else.

pub mod agenda_sent;
pub mod categories;
pub mod forms;
pub mod intake_queries;
pub mod intakes;
pub mod medicine_view;
pub mod medicines;
pub mod reminder_log;
pub mod slots;
pub mod stock;
pub mod symptoms;
pub mod sync_log;

use chassis::Error;
use chassis::storage::rusqlite;

/// Turn a sqlite failure into the kit's error, keeping the cause visible.
pub fn db_err(e: rusqlite::Error) -> Error {
    if e.sqlite_error_code() == Some(rusqlite::ErrorCode::ConstraintViolation) {
        // A foreign key or uniqueness rule: the request named something that
        // is not there (or is already there), which is the caller's to fix.
        return Error::invalid(
            "the request refers to something that does not exist or already exists",
            "reload the app so its lists are current, then try again",
        );
    }
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

#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_integrity;
