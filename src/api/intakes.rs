//! Logging doses, undoing them and the as-needed dosing guard.

use axum::Json;
use axum::extract::{Path, Query, State};
use chassis::storage::AppDb;
use serde::Deserialize;

use super::{ApiResult, Clock};
use crate::domain::date::Date;
use crate::domain::schedule::{DoseVerdict, PastDose, check_dose};
use crate::store::intakes::{self, Intake, IntakeInput};
use crate::store::medicines;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// A day (`YYYY-MM-DD`); every day when absent.
    day: Option<String>,
    limit: Option<i64>,
}

pub async fn list(State(db): State<AppDb>, Query(q): Query<ListQuery>) -> ApiResult<Vec<Intake>> {
    let prefix = q.day.unwrap_or_default();
    let limit = q.limit.unwrap_or(200).clamp(1, 1000);
    db.read(move |conn| intakes::list(conn, &prefix, limit))
        .await
        .map(Json)
}

pub async fn record(
    State(db): State<AppDb>,
    Json(input): Json<IntakeInput>,
) -> ApiResult<serde_json::Value> {
    Date::parse(&input.taken_at)?;
    let id = db.write(move |tx| intakes::record(tx, &input)).await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

pub async fn undo(State(db): State<AppDb>, Path(id): Path<i64>) -> ApiResult<serde_json::Value> {
    db.write(move |tx| intakes::undo(tx, id)).await?;
    Ok(Json(serde_json::json!({ "undone": id })))
}

/// May an as-needed medicine be taken right now?
pub async fn dose_check(
    State(db): State<AppDb>,
    Path(id): Path<i64>,
    Query(c): Query<Clock>,
) -> ApiResult<DoseVerdict> {
    let (today, minute) = (c.date()?, c.minute()?);
    let verdict = db
        .read(move |conn| {
            let m = medicines::get(conn, id, today)?;
            let ago =
                intakes::recent_taken_minutes_ago(conn, id, today.to_days(), i64::from(minute))?;
            let past: Vec<PastDose> = ago
                .into_iter()
                .map(|minutes_ago| PastDose { minutes_ago })
                .collect();
            Ok(check_dose(&past, m.min_gap_minutes, m.max_per_day))
        })
        .await?;
    Ok(Json(verdict))
}
