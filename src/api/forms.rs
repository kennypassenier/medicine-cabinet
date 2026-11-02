//! CRUD for the managed list of medicine forms.

use axum::Json;
use axum::extract::State;
use chassis::storage::AppDb;
use serde::Deserialize;

use super::ApiResult;
use super::extract::{ApiJson, ApiPath};
use crate::store::forms::{self, Form};

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
    ApiJson(b): ApiJson<FormBody>,
) -> ApiResult<serde_json::Value> {
    db.write(move |tx| forms::update(tx, id, &b.name, &b.unit))
        .await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

pub async fn remove(
    State(db): State<AppDb>,
    ApiPath(id): ApiPath<i64>,
) -> ApiResult<serde_json::Value> {
    let gone = db.write(move |tx| forms::delete(tx, id)).await?;
    Ok(Json(serde_json::json!({ "deleted": id, "was": gone })))
}
