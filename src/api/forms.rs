//! CRUD for the managed list of medicine forms.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use chassis::storage::AppDb;
use chassis::{Error, IfMatch, Revisioned};
use serde::Deserialize;

use super::ApiResult;
use super::guard::guarded;
use crate::store::forms::{self, Form};
use crate::store::sync_log::Coll;
use chassis::{ApiJson, ApiPath};

#[derive(Debug, Deserialize)]
pub struct FormBody {
    name: String,
    #[serde(default)]
    unit: String,
    /// Only on create: set this form on these medicines (the undo of a delete).
    #[serde(default)]
    medicine_ids: Vec<i64>,
}

pub async fn list(State(db): State<AppDb>) -> ApiResult<Vec<Form>> {
    db.read(forms::list).await.map(Json)
}

pub async fn create(
    State(db): State<AppDb>,
    ApiJson(b): ApiJson<FormBody>,
) -> ApiResult<serde_json::Value> {
    let id = db
        .write(move |tx| forms::create(tx, &b.name, &b.unit, &b.medicine_ids))
        .await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

pub async fn replace(
    State(db): State<AppDb>,
    ApiPath(id): ApiPath<i64>,
    if_match: IfMatch,
    ApiJson(b): ApiJson<FormBody>,
) -> Result<Response, Error> {
    let done = guarded(&db, if_match, Coll::Forms, id, move |tx| {
        forms::update(tx, id, &b.name, &b.unit)
    })
    .await?;
    Ok(match done {
        Err(conflict) => conflict.into_response(),
        Ok((rev, ())) => {
            Revisioned::new(rev, serde_json::json!({ "id": id, "rev": rev })).into_response()
        }
    })
}

pub async fn remove(
    State(db): State<AppDb>,
    ApiPath(id): ApiPath<i64>,
    if_match: IfMatch,
) -> Result<Response, Error> {
    let done = guarded(&db, if_match, Coll::Forms, id, move |tx| {
        forms::delete(tx, id)
    })
    .await?;
    Ok(match done {
        Err(conflict) => conflict.into_response(),
        Ok((rev, gone)) => Revisioned::new(
            rev,
            serde_json::json!({ "deleted": id, "rev": rev, "was": gone }),
        )
        .into_response(),
    })
}
