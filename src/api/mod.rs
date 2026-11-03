//! The JSON API the browser app talks to (behind the admin login).

mod forms;
mod guard;
mod intake_create;
mod intakes;
mod medicines;
mod search;
mod symptoms;
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
        .route("/api/mc/medicines/{id}/restore", post(medicines::restore))
        .route("/api/mc/medicines/{id}/stock", post(medicines::add_stock))
        .route(
            "/api/mc/medicines/{id}/dose-check",
            get(intakes::dose_check),
        )
        .route(
            "/api/mc/intakes",
            get(intakes::list).post(intake_create::record),
        )
        .route(
            "/api/mc/intakes/{id}",
            axum::routing::put(intakes::edit).delete(intakes::undo),
        )
        .route("/api/mc/today", get(today::overview))
        .route("/api/mc/search", get(search::by_symptom))
        .route(
            "/api/mc/symptoms",
            get(symptoms::list).post(symptoms::create),
        )
        .route(
            "/api/mc/symptoms/{id}",
            axum::routing::put(symptoms::replace).delete(symptoms::remove),
        )
        .route(
            "/api/mc/categories",
            get(symptoms::list_categories).post(symptoms::create_category),
        )
        .route(
            "/api/mc/categories/{id}",
            axum::routing::put(symptoms::rename_category).delete(symptoms::remove_category),
        )
        .route("/api/mc/forms", get(forms::list).post(forms::create))
        .route(
            "/api/mc/forms/{id}",
            axum::routing::put(forms::replace).delete(forms::remove),
        )
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
