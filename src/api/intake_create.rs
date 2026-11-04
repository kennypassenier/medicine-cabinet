//! Creating an intake, possibly offline: the browser may send the `uid` it
//! made and an `Idempotency-Key`; a replayed request is answered with the
//! intake it created, not taken twice (the stock must move once).

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use chassis::storage::AppDb;
use chassis::storage::rusqlite::Transaction;
use chassis::storage::sync::{CreateVerdict, KeyExists, check_create, client_key};
use chassis::{ApiJson, Error, IdempotencyKey};
use serde::Deserialize;
use uuid::Uuid;

use crate::store::intakes::{self, IntakeInput, Recorded};
use crate::store::refs;
use crate::store::sync_log::Coll;

#[derive(Debug, Deserialize)]
pub struct IntakeBody {
    #[serde(flatten)]
    input: IntakeInput,
    /// The key the browser made with `crypto.randomUUID()`; absent: the server makes one.
    uid: Option<String>,
    /// The `id` of the medicine taken.
    medicine_id: String,
}

pub async fn record(
    State(db): State<AppDb>,
    key: IdempotencyKey,
    ApiJson(body): ApiJson<IntakeBody>,
) -> Result<Response, Error> {
    let uid = body
        .uid
        .as_deref()
        .map(client_key)
        .transpose()?
        .map(|u| u.to_string());
    let sent = key.get();
    let outcome = db
        .write(move |tx| {
            let mut input = body.input;
            input.medicine_id = refs::field(tx, Coll::Medicines, "medicine_id", &body.medicine_id)?;
            create(tx, &input, uid.as_deref(), sent)
        })
        .await?;
    Ok(match outcome {
        Ok(r) => {
            Json(serde_json::json!({ "id": r.id, "uid": r.uid, "rev": r.rev })).into_response()
        }
        Err(exists) => exists.into_response(),
    })
}

/// The kit's create rule over this app's tables: new key creates, the same
/// request again replays, a key held by another request is refused.
fn create(
    tx: &Transaction,
    input: &IntakeInput,
    uid: Option<&str>,
    sent: Option<Uuid>,
) -> Result<Result<Recorded, KeyExists>, Error> {
    let idempotency = sent.map(|k| k.to_string());
    let held = intakes::find_held(tx, uid, idempotency.as_deref())?;
    let recorded = held
        .as_ref()
        .and_then(|h| h.idempotency_key.as_deref())
        .and_then(|k| client_key(k).ok());
    match (check_create(held.is_some(), sent, recorded), held) {
        (Ok(CreateVerdict::Create), _) => {
            intakes::record_keyed(tx, input, uid, idempotency.as_deref()).map(Ok)
        }
        (Ok(CreateVerdict::Replay), Some(h)) => Ok(Ok(Recorded {
            id: h.id,
            uid: h.uid,
            rev: h.rev,
        })),
        (Err(exists), _) => Ok(Err(exists)),
        (Ok(_), _) => Err(Error::internal(
            "the create rule answered something this app does not know",
            "report this: the kit's CreateVerdict gained a case",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::medicine::MedicineInput;
    use crate::store::medicines;
    use crate::testkit::sync_db;

    const UID: &str = "3f2b8c1e-5a4d-4e0a-9b1c-2d7e6f8a9b0c";

    fn key(s: &str) -> Option<Uuid> {
        Some(Uuid::parse_str(s).unwrap())
    }

    fn input(medicine_id: i64) -> IntakeInput {
        IntakeInput {
            medicine_id,
            taken_at: "2026-11-01T08:00".into(),
            planned_for: None,
            status: "taken".into(),
            dose: Some(1.0),
            note: String::new(),
        }
    }

    fn stock(tx: &Transaction, id: i64) -> f64 {
        crate::domain::quantity::to_decimal(crate::store::stock::hundredths(tx, id).unwrap())
    }

    fn medicine(tx: &Transaction) -> i64 {
        let m: MedicineInput = serde_json::from_value(serde_json::json!(
            { "name": "A", "stock": 5.0 }
        ))
        .unwrap();
        medicines::insert(tx, &m).unwrap()
    }

    #[test]
    fn a_replay_returns_the_first_intake_and_takes_no_second_dose() {
        let mut conn = sync_db();
        let tx = conn.transaction().unwrap();
        let id = medicine(&tx);
        let k = key("0b9d1a52-7c1e-4d38-8a55-1f0e2d3c4b5a");
        let first = create(&tx, &input(id), Some(UID), k).unwrap().unwrap();
        let again = create(&tx, &input(id), Some(UID), k).unwrap().unwrap();
        assert_eq!(first, again);
        assert_eq!(stock(&tx, id), 4.0, "the stock moved once");
    }

    #[test]
    fn a_held_uid_without_the_same_key_is_refused() {
        let mut conn = sync_db();
        let tx = conn.transaction().unwrap();
        let id = medicine(&tx);
        let k = key("0b9d1a52-7c1e-4d38-8a55-1f0e2d3c4b5a");
        create(&tx, &input(id), Some(UID), k).unwrap().unwrap();
        let other = key("11111111-7c1e-4d38-8a55-1f0e2d3c4b5a");
        assert!(create(&tx, &input(id), Some(UID), other).unwrap().is_err());
        assert!(create(&tx, &input(id), Some(UID), None).unwrap().is_err());
        assert_eq!(stock(&tx, id), 4.0);
    }

    #[test]
    fn without_a_client_uid_the_server_makes_one_and_the_key_still_replays() {
        let mut conn = sync_db();
        let tx = conn.transaction().unwrap();
        let id = medicine(&tx);
        let k = key("0b9d1a52-7c1e-4d38-8a55-1f0e2d3c4b5a");
        let first = create(&tx, &input(id), None, k).unwrap().unwrap();
        assert_eq!(first.uid.len(), 36);
        let again = create(&tx, &input(id), None, k).unwrap().unwrap();
        assert_eq!(first, again);
        assert_eq!(stock(&tx, id), 4.0);
    }
}
