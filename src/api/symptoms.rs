//! Symptom and category CRUD.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use chassis::storage::AppDb;
use chassis::{Error, IfMatch, Revisioned};
use serde::Deserialize;

use super::guard::guarded;
use super::{ApiResult, create};
use crate::store::categories::{self, Category};
use crate::store::refs;
use crate::store::symptoms::{self, Symptom};
use crate::store::sync_log::Coll;
use chassis::{ApiJson, ApiPath, IdempotencyKey};

#[derive(Debug, Deserialize)]
pub struct SymptomBody {
    name: String,
    /// Only on create: the `id` the browser made (a UUID); absent: the server makes one.
    id: Option<String>,
    /// The `id` of the category.
    category_id: Option<String>,
    /// Only on create: relink to these medicines (the `id`s; the undo of a delete).
    #[serde(default)]
    medicine_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct CategoryBody {
    name: String,
    /// Only on create: the `id` the browser made (a UUID); absent: the server makes one.
    id: Option<String>,
    /// Only on create: file these symptoms under it (the `id`s; the undo of a delete).
    #[serde(default)]
    symptom_ids: Vec<String>,
}

pub async fn list(State(db): State<AppDb>) -> ApiResult<Vec<Symptom>> {
    db.read(symptoms::list).await.map(Json)
}

pub async fn create(
    State(db): State<AppDb>,
    key: IdempotencyKey,
    ApiJson(b): ApiJson<SymptomBody>,
) -> Result<Response, Error> {
    let uid = create::client_uid(b.id.as_deref())?;
    let outcome = db
        .write(move |tx| {
            create::keyed(tx, Coll::Symptoms, uid.as_deref(), key.get(), |tx, keys| {
                let category = refs::field_opt(
                    tx,
                    Coll::Categories,
                    "category_id",
                    b.category_id.as_deref(),
                )?;
                let medicines =
                    refs::field_all(tx, Coll::Medicines, "medicine_ids", &b.medicine_ids)?;
                symptoms::create_keyed(tx, &b.name, category, &medicines, keys)
            })
        })
        .await?;
    Ok(create::respond(outcome))
}

pub async fn replace(
    State(db): State<AppDb>,
    ApiPath(reference): ApiPath<String>,
    if_match: IfMatch,
    ApiJson(b): ApiJson<SymptomBody>,
) -> Result<Response, Error> {
    let done = guarded(&db, if_match, Coll::Symptoms, reference, move |tx, id| {
        let category = refs::field_opt(
            tx,
            Coll::Categories,
            "category_id",
            b.category_id.as_deref(),
        )?;
        symptoms::update(tx, id, &b.name, category)
    })
    .await?;
    Ok(match done {
        Err(conflict) => conflict.into_response(),
        Ok(d) => {
            Revisioned::new(d.rev, serde_json::json!({ "id": d.key, "rev": d.rev })).into_response()
        }
    })
}

pub async fn remove(
    State(db): State<AppDb>,
    ApiPath(reference): ApiPath<String>,
    if_match: IfMatch,
) -> Result<Response, Error> {
    let done = guarded(&db, if_match, Coll::Symptoms, reference, symptoms::delete).await?;
    Ok(match done {
        Err(conflict) => conflict.into_response(),
        Ok(d) => Revisioned::new(
            d.rev,
            serde_json::json!({ "deleted": d.key, "rev": d.rev, "was": d.value }),
        )
        .into_response(),
    })
}

pub async fn list_categories(State(db): State<AppDb>) -> ApiResult<Vec<Category>> {
    db.read(categories::list).await.map(Json)
}

pub async fn create_category(
    State(db): State<AppDb>,
    key: IdempotencyKey,
    ApiJson(b): ApiJson<CategoryBody>,
) -> Result<Response, Error> {
    let uid = create::client_uid(b.id.as_deref())?;
    let outcome = db
        .write(move |tx| {
            create::keyed(
                tx,
                Coll::Categories,
                uid.as_deref(),
                key.get(),
                |tx, keys| {
                    let members =
                        refs::field_all(tx, Coll::Symptoms, "symptom_ids", &b.symptom_ids)?;
                    categories::create_keyed(tx, &b.name, &members, keys)
                },
            )
        })
        .await?;
    Ok(create::respond(outcome))
}

pub async fn rename_category(
    State(db): State<AppDb>,
    ApiPath(reference): ApiPath<String>,
    if_match: IfMatch,
    ApiJson(b): ApiJson<CategoryBody>,
) -> Result<Response, Error> {
    let done = guarded(&db, if_match, Coll::Categories, reference, move |tx, id| {
        categories::rename(tx, id, &b.name)
    })
    .await?;
    Ok(match done {
        Err(conflict) => conflict.into_response(),
        Ok(d) => {
            Revisioned::new(d.rev, serde_json::json!({ "id": d.key, "rev": d.rev })).into_response()
        }
    })
}

pub async fn remove_category(
    State(db): State<AppDb>,
    ApiPath(reference): ApiPath<String>,
    if_match: IfMatch,
) -> Result<Response, Error> {
    let done = guarded(
        &db,
        if_match,
        Coll::Categories,
        reference,
        categories::delete,
    )
    .await?;
    Ok(match done {
        Err(conflict) => conflict.into_response(),
        Ok(d) => Revisioned::new(
            d.rev,
            serde_json::json!({ "deleted": d.key, "rev": d.rev, "was": d.value }),
        )
        .into_response(),
    })
}
