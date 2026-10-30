//! The JSON API the browser app talks to (behind the admin login).

mod intakes;
mod medicines;
mod search;
mod today;

use axum::Router;
use axum::routing::{get, post};
use chassis::Error;
use chassis::storage::AppDb;
use serde::Deserialize;

use crate::domain::date::Date;

pub type ApiResult<T> = Result<axum::Json<T>, Error>;

pub fn router(db: AppDb) -> Router {
    Router::new()
        .route(
            "/api/mc/medicines",
            get(medicines::list).post(medicines::create),
        )
        .route(
            "/api/mc/medicines/{id}",
            get(medicines::show)
                .put(medicines::replace)
                .delete(medicines::remove),
        )
        .route("/api/mc/medicines/{id}/active", post(medicines::set_active))
        .route("/api/mc/medicines/{id}/stock", post(medicines::add_stock))
        .route(
            "/api/mc/medicines/{id}/dose-check",
            get(intakes::dose_check),
        )
        .route("/api/mc/intakes", get(intakes::list).post(intakes::record))
        .route("/api/mc/intakes/{id}", axum::routing::delete(intakes::undo))
        .route("/api/mc/today", get(today::overview))
        .route("/api/mc/search", get(search::by_symptom))
        .route("/api/mc/symptoms", get(search::symptoms))
        .with_state(db)
}

/// The browser's local clock, sent with every request that depends on it.
#[derive(Debug, Deserialize)]
pub struct Clock {
    /// Local `YYYY-MM-DDTHH:MM`.
    pub now: String,
}

impl Clock {
    pub fn date(&self) -> Result<Date, Error> {
        Date::parse(&self.now)
    }

    pub fn minute(&self) -> Result<u32, Error> {
        crate::domain::date::parse_minutes(&self.now)
    }
}
