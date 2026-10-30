//! The home screen: today's plan and what needs attention.

use axum::Json;
use axum::extract::{Query, State};
use chassis::storage::AppDb;
use serde::Serialize;

use super::{ApiResult, Clock};
use crate::attention::{Attention, attention_for};
use crate::domain::plan::{PlannedDose, plan};
use crate::store::intakes;
use crate::store::medicines::{self, Medicine};

#[derive(Debug, Serialize)]
pub struct Overview {
    pub doses: Vec<PlannedDose>,
    pub attention: Vec<Attention>,
    /// Switched-on as-needed medicines, for the quick "I took one" row.
    pub as_needed: Vec<Medicine>,
}

pub async fn overview(State(db): State<AppDb>, Query(c): Query<Clock>) -> ApiResult<Overview> {
    let (today, minute) = (c.date()?, c.minute()?);
    let day = today.iso();
    let (all, logged) = db
        .read(move |conn| {
            Ok((
                medicines::list(conn, today)?,
                intakes::list(conn, &day, 500)?,
            ))
        })
        .await?;
    let slots = crate::dayplan::day_slots(&all, &logged, today);
    let attention = attention_for(&all, today);
    let as_needed = all
        .into_iter()
        .filter(|m| m.active && m.as_needed && !m.course_over)
        .collect();
    Ok(Json(Overview {
        doses: plan(slots, minute),
        attention,
        as_needed,
    }))
}
