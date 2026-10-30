//! medicinecabinet — Medicines, schedules, stock and reminders
//!
//! Built on chassis: configuration, logging, health, metrics, graceful
//! shutdown, the dashboard with client tokens, signed self-update and
//! webhooks all come from the kit. This file holds only what medicinecabinet
//! does itself. Start with `cargo run -- --help`.

#![forbid(unsafe_code)]

use axum::routing::post;
use axum::{Json, Router};
use chassis::{App, AppSpec, Caller};

/// An example API route: anything a client posts with its token is
/// accepted and answered with who sent it. Replace with the real work.
async fn receive(caller: Caller, Json(body): Json<serde_json::Value>) -> Json<serde_json::Value> {
    let from = match caller {
        Caller::Client { name, .. } => name,
        Caller::Admin => "admin".to_string(),
    };
    Json(serde_json::json!({ "from": from, "received": body }))
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
        Err(e) => {
            eprintln!("{e}");
            return std::process::ExitCode::FAILURE;
        }
    };
    app.api_routes(Router::new().route("/v1/example", post(receive)));
    app.test_route(
        "POST",
        "/v1/example",
        "application/json",
        r#"{"hello":"world"}"#,
    );
    app.run().await
}
