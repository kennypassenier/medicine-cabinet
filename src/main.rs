//! medicine-cabinet — Medicines, schedules, stock and reminders
//!
//! Built on chassis: configuration, logging, health, graceful shutdown, the
//! admin login, the app database and the kp-themes assets come from the
//! kit. This file only wires the pieces together; the logic lives in
//! `domain` (pure rules), `store` (SQL) and `api` (JSON for the browser app).

#![forbid(unsafe_code)]

mod agenda;
mod api;
mod attention;
mod config;
mod dayplan;
mod domain;
mod reminders;
mod schema;
#[cfg(debug_assertions)]
mod seed;
mod store;
#[cfg(test)]
mod testkit;

use axum::Router;
use chassis::shell::pages::Page;
use chassis::shell::webapp::WebApp;
use chassis::storage::AppDbSpec;
use chassis::{App, AppSpec};

mod embedded {
    include!(concat!(env!("OUT_DIR"), "/web_files.rs"));
}

/// The browser app's areas, in navigation order: (id, title, path).
const PAGES: [(&str, &str, &str); 5] = [
    ("today", "Today", "/"),
    ("cabinet", "Cabinet", "/cabinet"),
    ("symptoms", "Symptoms", "/symptoms"),
    ("history", "History", "/history"),
    ("settings", "Settings", "/settings"),
];

fn fail(e: impl std::fmt::Display) -> std::process::ExitCode {
    eprintln!("{e}");
    std::process::ExitCode::FAILURE
}

#[tokio::main]
async fn main() -> std::process::ExitCode {
    let spec = AppSpec {
        name: "medicine-cabinet",
        version: env!("CARGO_PKG_VERSION"),
        repository: Some("kennypassenier/medicine-cabinet"),
        ..Default::default()
    };
    let mut app = match App::from_env_and_args(spec, Router::new()) {
        Ok(app) => app,
        Err(e) => return fail(e),
    };
    let reminder_config = match config::ReminderConfig::from_env() {
        Ok(c) => c,
        Err(e) => return fail(e),
    };
    let db = match app.app_db(AppDbSpec::new(schema::migrations())) {
        Ok(db) => db,
        Err(e) => return fail(e),
    };
    #[cfg(debug_assertions)]
    seed::spawn(db.clone());
    reminders::spawn(db.clone(), app.notifier(), reminder_config);
    if let Some(hub) = config::hub_from_env() {
        agenda::spawn(db.clone(), hub, reminder_config);
    }
    app.dashboard_routes(api::router(db));
    app.webapp(WebApp::embedded(embedded::FILES));
    app.brand_title("Medicine cabinet");
    // Registered so the kit's own pages (Status, Clients, Storage) draw the
    // same navigation as the browser app does.
    for (order, (id, title, path)) in PAGES.iter().enumerate() {
        let page = Page::new(id, title, path).order(order as i32);
        // Settings and the kit's admin pages live in the one More menu.
        app.page(if *id == "settings" {
            page.secondary()
        } else {
            page
        });
    }
    app.kit_page("status", |p| p.secondary());
    app.kit_page("clients", |p| p.secondary());
    app.run().await
}
