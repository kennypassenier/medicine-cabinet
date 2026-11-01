//! Symptom and category CRUD.

use axum::Json;
use axum::extract::{Path, State};
use chassis::storage::AppDb;
use serde::Deserialize;

use super::ApiResult;
use crate::store::categories::{self, Category};
use crate::store::symptoms::{self, Symptom};

#[derive(Debug, Deserialize)]
pub struct SymptomBody {
    name: String,
    category_id: Option<i64>,
    /// Only on create: relink to these medicines (the undo of a delete).
    #[serde(default)]
    medicine_ids: Vec<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CategoryBody {
    name: String,
    /// Only on create: file these symptoms under it (the undo of a delete).
    #[serde(default)]
    symptom_ids: Vec<i64>,
}

pub async fn list(State(db): State<AppDb>) -> ApiResult<Vec<Symptom>> {
    db.read(symptoms::list).await.map(Json)
}

pub async fn create(
    State(db): State<AppDb>,
    Json(b): Json<SymptomBody>,
) -> ApiResult<serde_json::Value> {
    let id = db
        .write(move |tx| symptoms::create(tx, &b.name, b.category_id, &b.medicine_ids))
        .await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

pub async fn replace(
    State(db): State<AppDb>,
    Path(id): Path<i64>,
    Json(b): Json<SymptomBody>,
) -> ApiResult<serde_json::Value> {
    db.write(move |tx| symptoms::update(tx, id, &b.name, b.category_id))
        .await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

pub async fn remove(State(db): State<AppDb>, Path(id): Path<i64>) -> ApiResult<serde_json::Value> {
    let gone = db.write(move |tx| symptoms::delete(tx, id)).await?;
    Ok(Json(serde_json::json!({ "deleted": id, "was": gone })))
}

pub async fn list_categories(State(db): State<AppDb>) -> ApiResult<Vec<Category>> {
    db.read(categories::list).await.map(Json)
}

pub async fn create_category(
    State(db): State<AppDb>,
    Json(b): Json<CategoryBody>,
) -> ApiResult<serde_json::Value> {
    let id = db
        .write(move |tx| categories::create(tx, &b.name, &b.symptom_ids))
        .await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

pub async fn rename_category(
    State(db): State<AppDb>,
    Path(id): Path<i64>,
    Json(b): Json<CategoryBody>,
) -> ApiResult<serde_json::Value> {
    db.write(move |tx| categories::rename(tx, id, &b.name))
        .await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

pub async fn remove_category(
    State(db): State<AppDb>,
    Path(id): Path<i64>,
) -> ApiResult<serde_json::Value> {
    let gone = db.write(move |tx| categories::delete(tx, id)).await?;
    Ok(Json(serde_json::json!({ "deleted": id, "was": gone })))
}
