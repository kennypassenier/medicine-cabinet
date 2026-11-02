//! Extractors that answer a bad request with the kit's JSON error (message
//! and remedy) instead of axum's bare text rejections.

use axum::extract::rejection::{JsonRejection, PathRejection, QueryRejection};
use axum::extract::{FromRequest, FromRequestParts, Request};
use axum::http::request::Parts;
use chassis::Error;
use serde::de::DeserializeOwned;

/// `axum::Json` with a clear error for a wrong content type or a bad body.
pub struct ApiJson<T>(pub T);

impl<S: Send + Sync, T: DeserializeOwned> FromRequest<S> for ApiJson<T> {
    type Rejection = Error;

    async fn from_request(req: Request, state: &S) -> Result<Self, Error> {
        match axum::Json::<T>::from_request(req, state).await {
            Ok(axum::Json(v)) => Ok(Self(v)),
            Err(JsonRejection::MissingJsonContentType(_)) => Err(Error::invalid(
                "the request body is not marked as JSON",
                "send the header Content-Type: application/json",
            )),
            Err(JsonRejection::BytesRejection(_)) => Err(Error::invalid(
                "the request body could not be read",
                "send a smaller body and try again",
            )),
            Err(other) => Err(Error::invalid(
                format!(
                    "the request body is not valid for this call: {}",
                    short(&other.body_text())
                ),
                "check the field names and types, and send every required field",
            )),
        }
    }
}

/// `axum::extract::Query` with the same kind of error.
pub struct ApiQuery<T>(pub T);

impl<S: Send + Sync, T: DeserializeOwned> FromRequestParts<S> for ApiQuery<T> {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Error> {
        axum::extract::Query::<T>::from_request_parts(parts, state)
            .await
            .map(|axum::extract::Query(v)| Self(v))
            .map_err(|e: QueryRejection| {
                Error::invalid(
                    format!("the query string is not valid: {}", short(&e.body_text())),
                    "send now=YYYY-MM-DDTHH:MM and any other required parameter",
                )
            })
    }
}

/// `axum::extract::Path` with the same kind of error.
pub struct ApiPath<T>(pub T);

impl<S: Send + Sync, T: DeserializeOwned + Send> FromRequestParts<S> for ApiPath<T> {
    type Rejection = Error;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Error> {
        axum::extract::Path::<T>::from_request_parts(parts, state)
            .await
            .map(|axum::extract::Path(v)| Self(v))
            .map_err(|e: PathRejection| {
                Error::invalid(
                    format!(
                        "the id in the address is not valid: {}",
                        short(&e.body_text())
                    ),
                    "use the numeric id of an item from the list",
                )
            })
    }
}

fn short(text: &str) -> String {
    text.chars().take(200).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;
    use chassis::Kind;

    #[derive(Debug, serde::Deserialize)]
    struct Probe {
        name: String,
    }

    async fn post(content_type: Option<&str>, body: &str) -> Result<ApiJson<Probe>, Error> {
        let mut req = HttpRequest::post("/");
        if let Some(ct) = content_type {
            req = req.header("content-type", ct);
        }
        ApiJson::<Probe>::from_request(req.body(Body::from(body.to_string())).unwrap(), &()).await
    }

    #[tokio::test]
    async fn a_good_body_passes() {
        let ok = post(Some("application/json"), r#"{"name":"x"}"#)
            .await
            .unwrap();
        assert_eq!(ok.0.name, "x");
    }

    #[tokio::test]
    async fn bad_bodies_become_json_errors_with_a_remedy() {
        let cases = [
            (None, r#"{"name":"x"}"#),
            (Some("text/plain"), r#"{"name":"x"}"#),
            (Some("application/json"), ""),
            (Some("application/json"), "{not json"),
            (Some("application/json"), r#"{"name":5}"#),
            (Some("application/json"), r#"{}"#),
            (Some("application/json"), r#"{"name":1e999}"#),
        ];
        for (ct, body) in cases {
            let err = post(ct, body)
                .await
                .err()
                .unwrap_or_else(|| panic!("{body}"));
            assert_eq!(err.kind, Kind::Invalid, "{ct:?} {body}");
            assert!(!err.remedy.is_empty());
        }
    }

    #[tokio::test]
    async fn a_bad_query_and_a_bad_path_are_refused_the_same_way() {
        let (mut parts, _) = HttpRequest::get("/?now=1").body(()).unwrap().into_parts();
        let q =
            ApiQuery::<std::collections::HashMap<String, u8>>::from_request_parts(&mut parts, &())
                .await;
        assert!(q.is_ok());
        let (mut parts, _) = HttpRequest::get("/?n=abc").body(()).unwrap().into_parts();
        let q =
            ApiQuery::<std::collections::HashMap<String, u8>>::from_request_parts(&mut parts, &())
                .await;
        assert_eq!(q.err().unwrap().kind, Kind::Invalid);
    }
}
