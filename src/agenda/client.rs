//! The HTTP side of the calendar hub (Almanac): upsert and delete by external id.

use std::time::Duration;

use chassis::Error;
use reqwest::Client;

use super::events::Wanted;

/// Where and how to reach the hub.
#[derive(Debug, Clone)]
pub struct HubConfig {
    /// Base address, no trailing slash, e.g. `http://10.10.10.12:8080`.
    pub base_url: String,
    /// The source id Almanac knows this app by (the URL segment).
    pub source_id: String,
    /// The source's client token.
    pub token: String,
}

const TIMEOUT: Duration = Duration::from_secs(15);

pub struct Hub {
    http: Client,
    config: HubConfig,
}

fn unreachable(what: &str, e: impl std::fmt::Display) -> Error {
    Error::dependency(
        format!("The calendar hub did not accept {what}: {e}"),
        "check the Almanac address and that its token for this source is valid; the next pass retries",
    )
}

impl Hub {
    pub fn new(config: HubConfig) -> Result<Self, Error> {
        let http = Client::builder().timeout(TIMEOUT).build().map_err(|e| {
            Error::internal(
                format!("the HTTP client could not start: {e}"),
                "this is a bug; report it",
            )
        })?;
        Ok(Self { http, config })
    }

    fn url(&self, tail: &str) -> String {
        format!(
            "{}/v1/ingest/{}{tail}",
            self.config.base_url, self.config.source_id
        )
    }

    /// Create or update the event for this external id.
    pub async fn upsert(&self, event: &Wanted) -> Result<(), Error> {
        let sent = self
            .http
            .post(self.url(""))
            .bearer_auth(&self.config.token)
            .json(event)
            .send()
            .await
            .map_err(|e| unreachable(&event.external_id, e))?;
        sent.error_for_status()
            .map(|_| ())
            .map_err(|e| unreachable(&event.external_id, e))
    }

    /// Remove the event; a 404 means it is already gone, which is fine.
    pub async fn delete(&self, external_id: &str) -> Result<(), Error> {
        let encoded = external_id.replace('/', "%2F");
        let sent = self
            .http
            .delete(self.url(&format!("/events/{encoded}")))
            .bearer_auth(&self.config.token)
            .send()
            .await
            .map_err(|e| unreachable(external_id, e))?;
        if sent.status() == reqwest::StatusCode::NOT_FOUND {
            return Ok(());
        }
        sent.error_for_status()
            .map(|_| ())
            .map_err(|e| unreachable(external_id, e))
    }
}
