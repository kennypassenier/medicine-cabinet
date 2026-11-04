//! Merged page endpoints: everything one screen needs in a single call.
//!
//! `GET /api/mc/pages/{name}?now=...` answers an object whose parts are the
//! JSON of the individual endpoints (which stay), read in ONE read
//! transaction, so the parts are consistent with each other. The parts come
//! from the same query functions the individual handlers use, and are
//! serialised straight from the typed structs (no intermediate JSON value),
//! so each part is byte-identical to its individual endpoint.

use axum::Json;
use axum::extract::State;
use axum::response::{IntoResponse, Response};
use chassis::storage::AppDb;
use chassis::storage::rusqlite::Connection;
use chassis::{ApiPath, ApiQuery, Error, Kind};
use serde::Serialize;

use super::Clock;
use super::intakes::DEFAULT_LIMIT;
use super::today::{self, Overview};
use crate::domain::date::Date;
use crate::store::categories::{self, Category};
use crate::store::forms::{self, Form};
use crate::store::intakes::{self, Intake};
use crate::store::medicines::{self, Medicine};
use crate::store::symptoms::{self, Symptom};

/// The pages that have a merged endpoint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Today,
    Cabinet,
    Symptoms,
    History,
    Settings,
}

impl Page {
    fn parse(name: &str) -> Result<Self, Error> {
        match name {
            "today" => Ok(Self::Today),
            "cabinet" => Ok(Self::Cabinet),
            "symptoms" => Ok(Self::Symptoms),
            "history" => Ok(Self::History),
            "settings" => Ok(Self::Settings),
            _ => Err(Error::new(
                Kind::NotFound,
                format!("there is no page endpoint called '{name}'"),
                "use today, cabinet, symptoms, history or settings",
            )),
        }
    }
}

#[derive(Debug, Serialize)]
struct TodayPage {
    today: Overview,
    medicines: Vec<Medicine>,
}

#[derive(Debug, Serialize)]
struct CabinetPage {
    medicines: Vec<Medicine>,
    forms: Vec<Form>,
    categories: Vec<Category>,
}

#[derive(Debug, Serialize)]
struct SymptomsPage {
    symptoms: Vec<Symptom>,
    categories: Vec<Category>,
    medicines: Vec<Medicine>,
}

#[derive(Debug, Serialize)]
struct HistoryPage {
    intakes: Vec<Intake>,
    medicines: Vec<Medicine>,
}

#[derive(Debug, Serialize)]
struct SettingsPage {
    forms: Vec<Form>,
    categories: Vec<Category>,
    symptoms: Vec<Symptom>,
}

/// Run `fetch` in one read transaction and answer its page as JSON.
async fn answer<T, F>(db: &AppDb, fetch: F) -> Result<Response, Error>
where
    T: Serialize + Send + 'static,
    F: FnOnce(&Connection) -> Result<T, Error> + Send + 'static,
{
    Ok(Json(db.read(fetch).await?).into_response())
}

fn cabinet(conn: &Connection, today: Date) -> Result<CabinetPage, Error> {
    Ok(CabinetPage {
        medicines: medicines::list(conn, today)?,
        forms: forms::list(conn)?,
        categories: categories::list(conn)?,
    })
}

fn symptoms_page(conn: &Connection, today: Date) -> Result<SymptomsPage, Error> {
    Ok(SymptomsPage {
        symptoms: symptoms::list(conn)?,
        categories: categories::list(conn)?,
        medicines: medicines::list(conn, today)?,
    })
}

fn history(conn: &Connection, today: Date) -> Result<HistoryPage, Error> {
    Ok(HistoryPage {
        intakes: intakes::list(conn, "", DEFAULT_LIMIT)?,
        medicines: medicines::list(conn, today)?,
    })
}

fn settings(conn: &Connection) -> Result<SettingsPage, Error> {
    Ok(SettingsPage {
        forms: forms::list(conn)?,
        categories: categories::list(conn)?,
        symptoms: symptoms::list(conn)?,
    })
}

fn today_page(conn: &Connection, today: Date, minute: u32) -> Result<TodayPage, Error> {
    Ok(TodayPage {
        today: today::build(conn, today, minute)?,
        medicines: medicines::list(conn, today)?,
    })
}

pub async fn show(
    State(db): State<AppDb>,
    ApiPath(name): ApiPath<String>,
    ApiQuery(c): ApiQuery<Clock>,
) -> Result<Response, Error> {
    let page = Page::parse(&name)?;
    let (today, minute) = (c.date()?, c.minute()?);
    match page {
        Page::Today => answer(&db, move |c| today_page(c, today, minute)).await,
        Page::Cabinet => answer(&db, move |c| cabinet(c, today)).await,
        Page::Symptoms => answer(&db, move |c| symptoms_page(c, today)).await,
        Page::History => answer(&db, move |c| history(c, today)).await,
        Page::Settings => answer(&db, settings).await,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_page_name_parses() {
        for (name, page) in [
            ("today", Page::Today),
            ("cabinet", Page::Cabinet),
            ("symptoms", Page::Symptoms),
            ("history", Page::History),
            ("settings", Page::Settings),
        ] {
            assert_eq!(Page::parse(name).unwrap(), page);
        }
    }

    #[test]
    fn an_unknown_page_is_not_found_with_a_remedy() {
        let e = Page::parse("nope").unwrap_err();
        assert!(e.to_string().contains("nope"));
    }
}
