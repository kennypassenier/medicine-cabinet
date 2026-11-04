//! Medicine CRUD, switching on/off and stock top-ups.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use chassis::storage::AppDb;
use chassis::{Error, IfMatch, Revisioned};
use serde::Deserialize;

use super::guard::guarded;
use super::{ApiResult, Clock, create};
use crate::domain::medicine::MedicineInput;
use crate::domain::revision::stock_is_current;
use crate::domain::schedule::SlotInput;
use crate::store::medicines::{self, Medicine};
use crate::store::sync_log::{self, Coll};
use crate::store::{refs, slots, stock};
use chassis::{ApiJson, ApiPath, ApiQuery, IdempotencyKey};

/// A medicine together with its repeating schedule.
#[derive(Debug, Deserialize)]
pub struct MedicineBody {
    #[serde(flatten)]
    medicine: MedicineInput,
    #[serde(default)]
    slots: Vec<SlotInput>,
    /// Only on create: the `id` the browser made (a UUID); absent: the server makes one.
    id: Option<String>,
    /// The `id` of one of the managed forms.
    form_id: Option<String>,
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
    ApiPath(reference): ApiPath<String>,
    ApiQuery(c): ApiQuery<Clock>,
) -> ApiResult<Medicine> {
    let today = c.date()?;
    db.read(move |conn| {
        let id = refs::path(conn, Coll::Medicines, &reference)?;
        medicines::get(conn, id, today)
    })
    .await
    .map(Json)
}

/// Create a medicine, possibly under an `id` made offline. Answers `{id, rev}`;
/// the page reads the medicine itself from the feed.
pub async fn create(
    State(db): State<AppDb>,
    key: IdempotencyKey,
    ApiJson(body): ApiJson<MedicineBody>,
) -> Result<Response, Error> {
    let uid = create::client_uid(body.id.as_deref())?;
    let mut input = body.medicine.validated()?;
    let slot_list = validate_slots(body.slots)?;
    let form = body.form_id;
    let outcome = db
        .write(move |tx| {
            create::keyed(
                tx,
                Coll::Medicines,
                uid.as_deref(),
                key.get(),
                |tx, keys| {
                    input.form_id = refs::field_opt(tx, Coll::Forms, "form_id", form.as_deref())?;
                    let id = medicines::insert_keyed(tx, &input, keys)?;
                    slots::replace(tx, id, &slot_list)?;
                    Ok(id)
                },
            )
        })
        .await?;
    Ok(create::respond(outcome))
}

pub async fn replace(
    State(db): State<AppDb>,
    ApiPath(reference): ApiPath<String>,
    if_match: IfMatch,
    ApiQuery(c): ApiQuery<Clock>,
    ApiJson(body): ApiJson<MedicineBody>,
) -> Result<Response, Error> {
    let today = c.date()?;
    let mut input = body.medicine.validated()?;
    let slot_list = validate_slots(body.slots)?;
    let form = body.form_id;
    let done = guarded(&db, if_match, Coll::Medicines, reference, move |tx, id| {
        input.form_id = refs::field_opt(tx, Coll::Forms, "form_id", form.as_deref())?;
        // An edit made before a later dose keeps the stock the dose left.
        let rev = sync_log::current_rev(tx, Coll::Medicines, id)?.unwrap_or_default();
        let write_stock = stock_is_current(if_match.precondition(), rev);
        medicines::update(tx, id, &input, write_stock)?;
        slots::replace(tx, id, &slot_list)
    })
    .await?;
    match done {
        Err(conflict) => Ok(conflict.into_response()),
        Ok(done) => {
            let m = db
                .read(move |conn| medicines::get(conn, done.id, today))
                .await?;
            Ok(Revisioned::new(done.rev, m).into_response())
        }
    }
}

pub async fn remove(
    State(db): State<AppDb>,
    ApiPath(reference): ApiPath<String>,
    if_match: IfMatch,
) -> Result<Response, Error> {
    let done = guarded(&db, if_match, Coll::Medicines, reference, medicines::delete).await?;
    Ok(match done {
        Err(conflict) => conflict.into_response(),
        Ok(d) => Revisioned::new(d.rev, serde_json::json!({ "deleted": d.key, "rev": d.rev }))
            .into_response(),
    })
}

/// Bring back a medicine deleted in the last two weeks.
pub async fn restore(
    State(db): State<AppDb>,
    ApiPath(reference): ApiPath<String>,
    ApiQuery(c): ApiQuery<Clock>,
) -> ApiResult<Medicine> {
    let today = c.date()?;
    db.write(move |tx| {
        let id = refs::path(tx, Coll::Medicines, &reference)?;
        medicines::restore(tx, id)?;
        medicines::get(tx, id, today)
    })
    .await
    .map(Json)
}

#[derive(Debug, Deserialize)]
pub struct ActiveBody {
    active: bool,
}

pub async fn set_active(
    State(db): State<AppDb>,
    ApiPath(reference): ApiPath<String>,
    ApiQuery(c): ApiQuery<Clock>,
    ApiJson(body): ApiJson<ActiveBody>,
) -> ApiResult<Medicine> {
    let today = c.date()?;
    db.write(move |tx| {
        let id = refs::path(tx, Coll::Medicines, &reference)?;
        medicines::set_active(tx, id, body.active)?;
        medicines::get(tx, id, today)
    })
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
    ApiPath(reference): ApiPath<String>,
    ApiQuery(c): ApiQuery<Clock>,
    ApiJson(body): ApiJson<StockBody>,
) -> ApiResult<Medicine> {
    let today = c.date()?;
    db.write(move |tx| {
        let id = refs::path(tx, Coll::Medicines, &reference)?;
        stock::adjust_stock(tx, id, body.delta)?;
        medicines::get(tx, id, today)
    })
    .await
    .map(Json)
}

fn validate_slots(raw: Vec<SlotInput>) -> Result<Vec<SlotInput>, chassis::Error> {
    crate::domain::limits::list_len("the schedule", raw.len())?;
    raw.into_iter().map(SlotInput::validated).collect()
}
