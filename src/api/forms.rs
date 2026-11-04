//! CRUD for the managed list of medicine forms.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use chassis::storage::AppDb;
use chassis::{Error, IfMatch, Revisioned};
use serde::Deserialize;

use super::guard::guarded;
use super::{ApiResult, create};
use crate::store::forms::{self, Form};
use crate::store::refs;
use crate::store::sync_log::Coll;
use chassis::{ApiJson, ApiPath, IdempotencyKey};

#[derive(Debug, Deserialize)]
pub struct FormBody {
    name: String,
    #[serde(default)]
    unit: String,
    /// Only on create: the `id` the browser made (a UUID); absent: the server makes one.
    id: Option<String>,
    /// Only on create: set this form on these medicines (the `id`s; the undo of a delete).
    #[serde(default)]
    medicine_ids: Vec<String>,
}

pub async fn list(State(db): State<AppDb>) -> ApiResult<Vec<Form>> {
    db.read(forms::list).await.map(Json)
}

pub async fn create(
    State(db): State<AppDb>,
    key: IdempotencyKey,
    ApiJson(b): ApiJson<FormBody>,
) -> Result<Response, Error> {
    let uid = create::client_uid(b.id.as_deref())?;
    let outcome = db
        .write(move |tx| {
            create::keyed(tx, Coll::Forms, uid.as_deref(), key.get(), |tx, keys| {
                let medicines =
                    refs::field_all(tx, Coll::Medicines, "medicine_ids", &b.medicine_ids)?;
                forms::create_keyed(tx, &b.name, &b.unit, &medicines, keys)
            })
        })
        .await?;
    Ok(create::respond(outcome))
}

pub async fn replace(
    State(db): State<AppDb>,
    ApiPath(reference): ApiPath<String>,
    if_match: IfMatch,
    ApiJson(b): ApiJson<FormBody>,
) -> Result<Response, Error> {
    let done = guarded(&db, if_match, Coll::Forms, reference, move |tx, id| {
        forms::update(tx, id, &b.name, &b.unit)
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
    let done = guarded(&db, if_match, Coll::Forms, reference, forms::delete).await?;
    Ok(match done {
        Err(conflict) => conflict.into_response(),
        Ok(d) => Revisioned::new(
            d.rev,
            serde_json::json!({ "deleted": d.key, "rev": d.rev, "was": d.value }),
        )
        .into_response(),
    })
}
