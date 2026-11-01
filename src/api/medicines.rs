//! Medicine CRUD, switching on/off and stock top-ups.

use axum::Json;
use axum::extract::{Path, Query, State};
use chassis::storage::AppDb;
use serde::Deserialize;

use super::{ApiResult, Clock};
use crate::domain::medicine::MedicineInput;
use crate::domain::schedule::SlotInput;
use crate::store::medicines::{self, Medicine};
use crate::store::slots;

/// A medicine together with its repeating schedule.
#[derive(Debug, Deserialize)]
pub struct MedicineBody {
    #[serde(flatten)]
    medicine: MedicineInput,
    #[serde(default)]
    slots: Vec<SlotInput>,
}

pub async fn list(State(db): State<AppDb>, Query(c): Query<Clock>) -> ApiResult<Vec<Medicine>> {
    let today = c.date()?;
    db.read(move |conn| medicines::list(conn, today))
        .await
        .map(Json)
}

pub async fn show(
    State(db): State<AppDb>,
    Path(id): Path<i64>,
    Query(c): Query<Clock>,
) -> ApiResult<Medicine> {
    let today = c.date()?;
    db.read(move |conn| medicines::get(conn, id, today))
        .await
        .map(Json)
}

pub async fn create(
    State(db): State<AppDb>,
    Query(c): Query<Clock>,
    Json(body): Json<MedicineBody>,
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
    Path(id): Path<i64>,
    Query(c): Query<Clock>,
    Json(body): Json<MedicineBody>,
) -> ApiResult<Medicine> {
    let today = c.date()?;
    let input = body.medicine.validated()?;
    let slot_list = validate_slots(body.slots)?;
    db.write(move |tx| {
        medicines::update(tx, id, &input)?;
        slots::replace(tx, id, &slot_list)
    })
    .await?;
    db.read(move |conn| medicines::get(conn, id, today))
        .await
        .map(Json)
}

pub async fn remove(State(db): State<AppDb>, Path(id): Path<i64>) -> ApiResult<serde_json::Value> {
    db.write(move |tx| medicines::delete(tx, id)).await?;
    Ok(Json(serde_json::json!({ "deleted": id })))
}

/// Bring back a medicine deleted in the last two weeks.
pub async fn restore(
    State(db): State<AppDb>,
    Path(id): Path<i64>,
    Query(c): Query<Clock>,
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
    Path(id): Path<i64>,
    Query(c): Query<Clock>,
    Json(body): Json<ActiveBody>,
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
    Path(id): Path<i64>,
    Query(c): Query<Clock>,
    Json(body): Json<StockBody>,
) -> ApiResult<Medicine> {
    let today = c.date()?;
    db.write(move |tx| medicines::adjust_stock(tx, id, body.delta))
        .await?;
    db.read(move |conn| medicines::get(conn, id, today))
        .await
        .map(Json)
}

fn validate_slots(raw: Vec<SlotInput>) -> Result<Vec<SlotInput>, chassis::Error> {
    raw.into_iter().map(SlotInput::validated).collect()
}
