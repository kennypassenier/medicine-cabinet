//! Creating a medicine, symptom, category or form, possibly offline: the
//! browser may send the `id` it made (a UUID) and an `Idempotency-Key`; the
//! kit's create rule decides. A new uid creates, the same request again
//! replays the row it made (no second row, no second side effect), a uid held
//! by another request is a 409 `exists`.

use axum::Json;
use axum::response::{IntoResponse, Response};
use chassis::Error;
use chassis::storage::rusqlite::Transaction;
use chassis::storage::sync::{CreateVerdict, KeyExists, check_create, client_key};
use uuid::Uuid;

use crate::store::refs::{self, Keys};
use crate::store::sync_log::Coll;

/// What a create answers: the public id and the revision.
#[derive(Debug, Clone, PartialEq)]
pub struct Created {
    pub uid: String,
    pub rev: u64,
}

/// The `id` of a create body, checked: a well-formed UUID (400 otherwise),
/// in canonical form. `None` when the browser sent none.
pub fn client_uid(raw: Option<&str>) -> Result<Option<String>, Error> {
    raw.map(|r| client_key(r).map(|u| u.to_string()))
        .transpose()
}

/// Run the create rule over `coll`'s table. `make` inserts the row (with the
/// keys it is given) and returns its internal id; it runs only for a new uid.
pub fn keyed(
    tx: &Transaction,
    coll: Coll,
    uid: Option<&str>,
    sent: Option<Uuid>,
    make: impl FnOnce(&Transaction, &Keys) -> Result<i64, Error>,
) -> Result<Result<Created, KeyExists>, Error> {
    let idempotency = sent.map(|k| k.to_string());
    let held = refs::find_held(tx, coll, uid, idempotency.as_deref())?;
    let recorded = held
        .as_ref()
        .and_then(|h| h.idempotency_key.as_deref())
        .and_then(|k| client_key(k).ok());
    match (check_create(held.is_some(), sent, recorded), held) {
        (Ok(CreateVerdict::Create), _) => {
            let keys = Keys {
                uid,
                idempotency: idempotency.as_deref(),
            };
            let id = make(tx, &keys)?;
            created(tx, coll, id).map(Ok)
        }
        (Ok(CreateVerdict::Replay), Some(h)) => Ok(Ok(Created {
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

/// The uid and revision a freshly made row ended at (its later side effects
/// may have bumped the revision).
fn created(tx: &Transaction, coll: Coll, id: i64) -> Result<Created, Error> {
    Ok(Created {
        uid: refs::uid_of(tx, coll, id)?,
        rev: crate::store::sync_log::current_rev(tx, coll, id)?.unwrap_or(1),
    })
}

/// The answer to a create route: `{id: <uid>, rev}` or the 409.
pub fn respond(outcome: Result<Created, KeyExists>) -> Response {
    match outcome {
        Ok(c) => Json(serde_json::json!({ "id": c.uid, "rev": c.rev })).into_response(),
        Err(exists) => exists.into_response(),
    }
}

#[cfg(test)]
mod tests;
