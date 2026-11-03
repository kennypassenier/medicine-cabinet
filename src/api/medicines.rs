//! Medicine CRUD, switching on/off and stock top-ups.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use chassis::storage::AppDb;
use chassis::{Error, IfMatch, Revisioned};
use serde::Deserialize;

use super::guard::guarded;
use super::{ApiResult, Clock};
use crate::domain::medicine::MedicineInput;
use crate::domain::schedule::SlotInput;
use crate::store::medicines::{self, Medicine};
use crate::store::sync_log::Coll;
use crate::store::{slots, stock};
use chassis::{ApiJson, ApiPath, ApiQuery};

/// A medicine together with its repeating schedule.
#[derive(Debug, Deserialize)]
pub struct MedicineBody {
    #[serde(flatten)]
    medicine: MedicineInput,
    #[serde(default)]
    slots: Vec<SlotInput>,
}

pub async fn list(
    State(db): State<AppDb>,
    ApiQuery(c): ApiQuery<Clock>,
) -> ApiResult<Vec<Medicine>> {
    let today = c.date()?;
    db.read(move |conn| medicines::list(conn, today))
        .await
        .map(Json)
}

pub async fn show(
    State(db): State<AppDb>,
    ApiPath(id): ApiPath<i64>,
    ApiQuery(c): ApiQuery<Clock>,
) -> ApiResult<Medicine> {
    let today = c.date()?;
    db.read(move |conn| medicines::get(conn, id, today))
        .await
        .map(Json)
}

pub async fn create(
    State(db): State<AppDb>,
    ApiQuery(c): ApiQuery<Clock>,
    ApiJson(body): ApiJson<MedicineBody>,
) -> ApiResult<Medicine> {
    let today = c.date()?;
    let input = body.medicine.validated()?;
    let slot_list = validate_slots(body.slots)?;
    let id = db
        .write(move |tx| {
            let id = medicines::insert(tx, &input)?;
            slots::replace(tx, id, &slot_list)?;
            Ok(id)
        })
        .await?;
    db.read(move |conn| medicines::get(conn, id, today))
        .await
        .map(Json)
}

pub async fn replace(
    State(db): State<AppDb>,
    ApiPath(id): ApiPath<i64>,
    if_match: IfMatch,
    ApiQuery(c): ApiQuery<Clock>,
    ApiJson(body): ApiJson<MedicineBody>,
) -> Result<Response, Error> {
    let today = c.date()?;
    let input = body.medicine.validated()?;
    let slot_list = validate_slots(body.slots)?;
    let done = guarded(&db, if_match, Coll::Medicines, id, move |tx| {
        medicines::update(tx, id, &input)?;
        slots::replace(tx, id, &slot_list)
    })
    .await?;
    match done {
        Err(conflict) => Ok(conflict.into_response()),
        Ok((rev, ())) => {
            let m = db.read(move |conn| medicines::get(conn, id, today)).await?;
            Ok(Revisioned::new(rev, m).into_response())
        }
    }
}

pub async fn remove(
    State(db): State<AppDb>,
    ApiPath(id): ApiPath<i64>,
    if_match: IfMatch,
) -> Result<Response, Error> {
    let done = guarded(&db, if_match, Coll::Medicines, id, move |tx| {
        medicines::delete(tx, id)
    })
    .await?;
    Ok(match done {
        Err(conflict) => conflict.into_response(),
        Ok((rev, ())) => {
            Revisioned::new(rev, serde_json::json!({ "deleted": id, "rev": rev })).into_response()
        }
    })
}

/// Bring back a medicine deleted in the last two weeks.
pub async fn restore(
    State(db): State<AppDb>,
    ApiPath(id): ApiPath<i64>,
    ApiQuery(c): ApiQuery<Clock>,
) -> ApiResult<Medicine> {
    let today = c.date()?;
    db.write(move |tx| medicines::restore(tx, id)).await?;
    db.read(move |conn| medicines::get(conn, id, today))
        .await
        .map(Json)
}

#[derive(Debug, Deserialize)]
pub struct ActiveBody {
    active: bool,
}

pub async fn set_active(
    State(db): State<AppDb>,
    ApiPath(id): ApiPath<i64>,
    ApiQuery(c): ApiQuery<Clock>,
    ApiJson(body): ApiJson<ActiveBody>,
) -> ApiResult<Medicine> {
    let today = c.date()?;
    db.write(move |tx| medicines::set_active(tx, id, body.active))
        .await?;
    db.read(move |conn| medicines::get(conn, id, today))
        .await
        .map(Json)
}

#[derive(Debug, Deserialize)]
pub struct StockBody {
    /// Positive to add, negative to correct downwards.
    delta: f64,
}

pub async fn add_stock(
    State(db): State<AppDb>,
    ApiPath(id): ApiPath<i64>,
    ApiQuery(c): ApiQuery<Clock>,
    ApiJson(body): ApiJson<StockBody>,
) -> ApiResult<Medicine> {
    let today = c.date()?;
    db.write(move |tx| stock::adjust_stock(tx, id, body.delta))
        .await?;
    db.read(move |conn| medicines::get(conn, id, today))
        .await
        .map(Json)
}

fn validate_slots(raw: Vec<SlotInput>) -> Result<Vec<SlotInput>, chassis::Error> {
    crate::domain::limits::list_len("the schedule", raw.len())?;
    raw.into_iter().map(SlotInput::validated).collect()
}
