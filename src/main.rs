//! medicinecabinet — Medicines, schedules, stock and reminders
//!
//! Built on chassis: configuration, logging, health, graceful shutdown, the
//! admin login, the app database and the kp-themes assets come from the
//! kit. This file only wires the pieces together; the logic lives in
//! `domain` (pure rules), `store` (SQL) and `api` (JSON for the browser app).

#![forbid(unsafe_code)]

mod api;
mod domain;
mod schema;
mod store;

use axum::Router;
use chassis::shell::webapp::WebApp;
use chassis::storage::AppDbSpec;
use chassis::{App, AppSpec};

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/web_files.rs"));
}

fn fail(e: impl std::fmt::Display) -> std::process::ExitCode {
    eprintln!("{e}");
    std::process::ExitCode::FAILURE
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let spec = AppSpec {
        name: "medicinecabinet",
        version: env!("CARGO_PKG_VERSION"),
        repository: Some("kennypassenier/medicinecabinet"),
        ..Default::default()
    };
    let mut app = match App::from_env_and_args(spec, Router::new()) {
        Ok(app) => app,
        Err(e) => return fail(e),
    };
    let db = match app.app_db(AppDbSpec::new(schema::migrations())) {
        Ok(db) => db,
        Err(e) => return fail(e),
    };
    app.dashboard_routes(api::router(db));
    app.webapp(WebApp::embedded(embedded::FILES));
    app.brand_title("Medicine cabinet");
    app.run().await
}
