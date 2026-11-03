//! Logging doses, undoing them and the as-needed dosing guard.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use chassis::storage::AppDb;
use serde::Deserialize;

use super::{ApiResult, Clock};
use crate::domain::date::Date;
use crate::domain::schedule::{DoseVerdict, PastDose, check_dose};
use crate::store::intakes::{self, Intake, IntakeEdit};
use crate::store::medicines;
use crate::store::sync_log::Coll;
use chassis::{ApiJson, ApiPath, ApiQuery, Error, IfMatch, Revisioned};

use super::guard::guarded;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    /// A day (`YYYY-MM-DD`); every day when absent.
    day: Option<String>,
    limit: Option<i64>,
}

pub async fn list(
    State(db): State<AppDb>,
    ApiQuery(q): ApiQuery<ListQuery>,
) -> ApiResult<Vec<Intake>> {
    // A day is validated, so no pattern text can reach the query.
    let prefix = match q.day.as_deref().map(str::trim).filter(|d| !d.is_empty()) {
        Some(day) => Date::parse(day)?.iso(),
        None => String::new(),
    };
    let limit = q.limit.unwrap_or(200).clamp(1, 1000);
    db.read(move |conn| intakes::list(conn, &prefix, limit))
        .await
        .map(Json)
}

/// Correct the time, dose or note of a logged intake (addressed by id or `uid`).
pub async fn edit(
    State(db): State<AppDb>,
    ApiPath(reference): ApiPath<String>,
    if_match: IfMatch,
    ApiJson(edit): ApiJson<IntakeEdit>,
) -> Result<Response, Error> {
    let id = db.read(move |c| intakes::resolve(c, &reference)).await?;
    let done = guarded(&db, if_match, Coll::Intakes, id, move |tx| {
        intakes::edit(tx, id, &edit)
    })
    .await?;
    Ok(match done {
        Err(conflict) => conflict.into_response(),
        Ok((rev, ())) => {
            Revisioned::new(rev, serde_json::json!({ "id": id, "rev": rev })).into_response()
        }
    })
}

pub async fn undo(
    State(db): State<AppDb>,
    ApiPath(reference): ApiPath<String>,
) -> ApiResult<serde_json::Value> {
    let id = db.read(move |c| intakes::resolve(c, &reference)).await?;
    db.write(move |tx| intakes::undo(tx, id)).await?;
    Ok(Json(serde_json::json!({ "undone": id })))
}

/// May an as-needed medicine be taken right now?
pub async fn dose_check(
    State(db): State<AppDb>,
    ApiPath(id): ApiPath<i64>,
    ApiQuery(c): ApiQuery<Clock>,
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
