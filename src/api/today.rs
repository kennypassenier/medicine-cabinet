//! The home screen: today's plan and what needs attention.

use axum::Json;
use axum::extract::State;
use chassis::Error;
use chassis::storage::AppDb;
use chassis::storage::rusqlite::Connection;
use serde::Serialize;

use super::{ApiResult, Clock};
use crate::attention::{Attention, attention_for};
use crate::domain::date::Date;
use crate::domain::plan::{PlannedDose, plan};
use crate::store::intakes;
use crate::store::medicines::{self, Medicine};
use chassis::ApiQuery;

#[derive(Debug, Serialize)]
pub struct Overview {
    pub doses: Vec<PlannedDose>,
    pub attention: Vec<Attention>,
    /// Switched-on as-needed medicines, for the quick "I took one" row.
    pub as_needed: Vec<Medicine>,
}

pub async fn overview(
    State(db): State<AppDb>,
    ApiQuery(c): ApiQuery<Clock>,
) -> ApiResult<Overview> {
    let (today, minute) = (c.date()?, c.minute()?);
    db.read(move |conn| build(conn, today, minute))
        .await
        .map(Json)
}

/// Today's overview from an open connection (shared with the merged `today` page).
pub(super) fn build(conn: &Connection, today: Date, minute: u32) -> Result<Overview, Error> {
    let day = today.iso();
    let all = medicines::list(conn, today)?;
    let logged = intakes::list(conn, &day, 500)?;
    let slots = crate::dayplan::day_slots(&all, &logged, today);
    let attention = attention_for(&all, today);
    let as_needed = all
        .into_iter()
        .filter(|m| m.active && m.as_needed && !m.course_over)
        .collect();
    Ok(Overview {
        doses: plan(slots, minute),
        attention,
        as_needed,
    })
}
