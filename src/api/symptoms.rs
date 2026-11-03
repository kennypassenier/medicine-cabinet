//! Symptom and category CRUD.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use chassis::storage::AppDb;
use chassis::{Error, IfMatch, Revisioned};
use serde::Deserialize;

use super::ApiResult;
use super::guard::guarded;
use crate::store::categories::{self, Category};
use crate::store::symptoms::{self, Symptom};
use crate::store::sync_log::Coll;
use chassis::{ApiJson, ApiPath};

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
    ApiJson(b): ApiJson<SymptomBody>,
) -> ApiResult<serde_json::Value> {
    let id = db
        .write(move |tx| symptoms::create(tx, &b.name, b.category_id, &b.medicine_ids))
        .await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

pub async fn replace(
    State(db): State<AppDb>,
    ApiPath(id): ApiPath<i64>,
    if_match: IfMatch,
    ApiJson(b): ApiJson<SymptomBody>,
) -> Result<Response, Error> {
    let done = guarded(&db, if_match, Coll::Symptoms, id, move |tx| {
        symptoms::update(tx, id, &b.name, b.category_id)
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
    let done = guarded(&db, if_match, Coll::Symptoms, id, move |tx| {
        symptoms::delete(tx, id)
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

pub async fn list_categories(State(db): State<AppDb>) -> ApiResult<Vec<Category>> {
    db.read(categories::list).await.map(Json)
}

pub async fn create_category(
    State(db): State<AppDb>,
    ApiJson(b): ApiJson<CategoryBody>,
) -> ApiResult<serde_json::Value> {
    let id = db
        .write(move |tx| categories::create(tx, &b.name, &b.symptom_ids))
        .await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

pub async fn rename_category(
    State(db): State<AppDb>,
    ApiPath(id): ApiPath<i64>,
    if_match: IfMatch,
    ApiJson(b): ApiJson<CategoryBody>,
) -> Result<Response, Error> {
    let done = guarded(&db, if_match, Coll::Categories, id, move |tx| {
        categories::rename(tx, id, &b.name)
    })
    .await?;
    Ok(match done {
        Err(conflict) => conflict.into_response(),
        Ok((rev, ())) => {
            Revisioned::new(rev, serde_json::json!({ "id": id, "rev": rev })).into_response()
        }
    })
}

pub async fn remove_category(
    State(db): State<AppDb>,
    ApiPath(id): ApiPath<i64>,
    if_match: IfMatch,
) -> Result<Response, Error> {
    let done = guarded(&db, if_match, Coll::Categories, id, move |tx| {
        categories::delete(tx, id)
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
