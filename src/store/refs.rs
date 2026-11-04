//! The public identifier of medicines, symptoms, categories and forms.
//!
//! Over the API and the sync feed these four kinds are known by a `uid`
//! (UUID text) only, so a device can make one while offline. Inside Rust and
//! SQL the integer `id` stays the primary key and the foreign key. Everything
//! that turns one into the other lives here: the API resolves the uids a
//! request names to ids inside its transaction, and the stores read the uid
//! next to the id for the answer.

use chassis::storage::rusqlite::{Connection, OptionalExtension};
use chassis::storage::sync::client_key;
use chassis::{Error, Kind};

use super::db_err;
use super::intakes;
use super::sync_log::{self, Coll};

/// Canonical form of a uid a caller sent: a 36-character UUID, lowercase.
/// Anything else is a 400, whether it is a path parameter or a body field.
pub fn normalize(raw: &str) -> Result<String, Error> {
    client_key(raw).map(|u| u.to_string())
}

/// The internal id behind a uid; `None` when no row holds it (a
/// soft-deleted medicine is still a row).
pub fn lookup(conn: &Connection, coll: Coll, uid: &str) -> Result<Option<i64>, Error> {
    let sql = format!("SELECT id FROM {} WHERE uid = ?1", coll.table());
    conn.query_row(&sql, [uid], |r| r.get(0))
        .optional()
        .map_err(db_err)
}

fn missing(coll: Coll, uid: &str) -> Error {
    Error::new(
        Kind::NotFound,
        format!("the {} {uid} does not exist", coll.singular()),
        "reload the list; it may have been deleted",
    )
}

/// The id a route's `{id}` names: 400 when it is not a UUID, 404 when no row
/// holds it. An intake is also addressed by its numeric id (see
/// [`intakes::resolve`]).
pub fn path(conn: &Connection, coll: Coll, raw: &str) -> Result<i64, Error> {
    if coll == Coll::Intakes {
        return intakes::resolve(conn, raw);
    }
    let uid = normalize(raw)?;
    lookup(conn, coll, &uid)?.ok_or_else(|| missing(coll, &uid))
}

/// The id a request body field names: a 400 naming the field when the uid is
/// malformed or unknown (the request is wrong, not the URL).
pub fn field(conn: &Connection, coll: Coll, field: &str, raw: &str) -> Result<i64, Error> {
    let bad = |why: String| {
        Error::invalid(
            format!("`{field}`: {why}"),
            format!(
                "send the id of an existing {} as `{field}`",
                coll.singular()
            ),
        )
    };
    let uid = normalize(raw).map_err(|_| bad(format!("`{raw}` is not a UUID")))?;
    lookup(conn, coll, &uid)?.ok_or_else(|| bad(format!("no {} has the id {uid}", coll.singular())))
}

/// [`field`] for an optional reference.
pub fn field_opt(
    conn: &Connection,
    coll: Coll,
    name: &str,
    raw: Option<&str>,
) -> Result<Option<i64>, Error> {
    raw.map(|r| field(conn, coll, name, r)).transpose()
}

/// [`field`] for a list of references.
pub fn field_all(
    conn: &Connection,
    coll: Coll,
    name: &str,
    raw: &[String],
) -> Result<Vec<i64>, Error> {
    raw.iter().map(|r| field(conn, coll, name, r)).collect()
}

/// The uid of a row, for the answer to a request.
pub fn uid_of(conn: &Connection, coll: Coll, id: i64) -> Result<String, Error> {
    let sql = format!("SELECT uid FROM {} WHERE id = ?1", coll.table());
    conn.query_row(&sql, [id], |r| r.get(0)).map_err(db_err)
}

/// [`uid_of`] for several rows, in the order given.
pub fn uids_of(conn: &Connection, coll: Coll, ids: &[i64]) -> Result<Vec<String>, Error> {
    ids.iter().map(|id| uid_of(conn, coll, *id)).collect()
}

/// The keys a create request carried: the `uid` the browser made (the server
/// makes one when `None`) and the `Idempotency-Key` header, as text.
#[derive(Debug, Default, Clone, Copy)]
pub struct Keys<'a> {
    pub uid: Option<&'a str>,
    pub idempotency: Option<&'a str>,
}

/// A row a create request's `id` or `Idempotency-Key` already points at.
#[derive(Debug, Clone, PartialEq)]
pub struct Held {
    pub id: i64,
    pub uid: String,
    pub rev: u64,
    /// The `Idempotency-Key` it was created with, if one was sent then.
    pub idempotency_key: Option<String>,
}

/// The stored row for a create request: by the `uid` when the browser sent
/// one, else by the `Idempotency-Key` alone.
pub fn find_held(
    conn: &Connection,
    coll: Coll,
    uid: Option<&str>,
    idempotency_key: Option<&str>,
) -> Result<Option<Held>, Error> {
    let (column, value) = match (uid, idempotency_key) {
        (Some(uid), _) => ("uid", uid),
        (None, Some(key)) => ("idempotency_key", key),
        (None, None) => return Ok(None),
    };
    let sql = format!(
        "SELECT id, uid, rev, idempotency_key FROM {} WHERE {column} = ?1",
        coll.table()
    );
    conn.query_row(&sql, [value], |r| {
        Ok(Held {
            id: r.get(0)?,
            uid: r.get(1)?,
            rev: sync_log::read_rev(r, 2)?,
            idempotency_key: r.get(3)?,
        })
    })
    .optional()
    .map_err(db_err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::forms;
    use crate::testkit::sync_db;

    #[test]
    fn a_uid_resolves_to_its_id_and_back() {
        let mut conn = sync_db();
        let tx = conn.transaction().unwrap();
        let id = forms::create(&tx, "Gel", "g", &[]).unwrap();
        let uid = uid_of(&tx, Coll::Forms, id).unwrap();
        assert_eq!(path(&tx, Coll::Forms, &uid.to_uppercase()).unwrap(), id);
        assert_eq!(uids_of(&tx, Coll::Forms, &[id]).unwrap(), [uid]);
    }

    #[test]
    fn a_path_is_400_when_malformed_and_404_when_unknown() {
        let mut conn = sync_db();
        let tx = conn.transaction().unwrap();
        let bad = path(&tx, Coll::Forms, "7").unwrap_err();
        assert_eq!(bad.kind, Kind::Invalid);
        let unknown = path(&tx, Coll::Forms, "3f2b8c1e-5a4d-4e0a-9b1c-2d7e6f8a9b0c").unwrap_err();
        assert_eq!(unknown.kind, Kind::NotFound);
    }

    #[test]
    fn a_body_field_names_itself_when_the_reference_is_wrong() {
        let mut conn = sync_db();
        let tx = conn.transaction().unwrap();
        let e = field(
            &tx,
            Coll::Forms,
            "form_id",
            "3f2b8c1e-5a4d-4e0a-9b1c-2d7e6f8a9b0c",
        )
        .unwrap_err();
        assert_eq!(e.kind, Kind::Invalid);
        assert!(e.message.contains("`form_id`"));
        assert!(
            field(&tx, Coll::Forms, "form_id", "nope")
                .unwrap_err()
                .message
                .contains("`form_id`")
        );
    }
}
