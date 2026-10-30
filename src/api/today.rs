//! The home screen: today's plan and what needs attention.

use axum::Json;
use axum::extract::{Query, State};
use chassis::storage::AppDb;
use serde::Serialize;

use super::{ApiResult, Clock};
use crate::domain::date::Date;
use crate::domain::medicine::{Freshness, StockLevel};
use crate::domain::plan::{PlanSlot, PlannedDose, plan};
use crate::store::intakes;
use crate::store::medicines::{self, Medicine};

#[derive(Debug, Serialize)]
pub struct Attention {
    pub medicine_id: i64,
    pub medicine_name: String,
    /// `out_of_stock`, `low_stock`, `expired`, `expiring_soon`, `course_ends_soon`.
    pub kind: &'static str,
    pub detail: String,
}

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
    let slots = day_slots(&all, &logged, today);
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

/// Scheduled slots that run today, paired with the intake that answered them.
fn day_slots(all: &[Medicine], logged: &[intakes::Intake], today: Date) -> Vec<PlanSlot> {
    let mut out = Vec::new();
    for m in all
        .iter()
        .filter(|m| m.active && !m.as_needed && !m.course_over)
    {
        for s in m.slots.iter().filter(|s| s.runs_on(today)) {
            let planned = format!("{}T{}", today.iso(), s.time);
            let answered = logged
                .iter()
                .find(|i| i.medicine_id == m.id && i.planned_for.as_deref() == Some(&planned))
                .map(|i| (i.id, i.status.clone()));
            out.push(PlanSlot {
                medicine_id: m.id,
                medicine_name: m.name.clone(),
                minute: crate::domain::date::parse_minutes(&s.time).unwrap_or(0),
                time: s.time.clone(),
                dose: s.dose,
                answered,
            });
        }
    }
    out
}

fn attention_for(all: &[Medicine], today: Date) -> Vec<Attention> {
    let mut out = Vec::new();
    for m in all.iter().filter(|m| m.active) {
        let mut push = |kind, detail: String| {
            out.push(Attention {
                medicine_id: m.id,
                medicine_name: m.name.clone(),
                kind,
                detail,
            });
        };
        match m.stock_level {
            StockLevel::Out => push("out_of_stock", "None left.".into()),
            StockLevel::Low => push(
                "low_stock",
                format!("Only {} {} left.", m.stock, m.stock_unit),
            ),
            StockLevel::Ok => {}
        }
        match (m.freshness, &m.effective_expiry) {
            (Freshness::Expired, Some(d)) => push("expired", format!("Expired on {d}.")),
            (Freshness::ExpiringSoon, Some(d)) => push("expiring_soon", format!("Good until {d}.")),
            _ => {}
        }
        if let Some(end) = m
            .course_ends_on
            .as_deref()
            .and_then(|d| Date::parse(d).ok())
        {
            let left = today.days_until(end);
            if (0..=3).contains(&left) {
                push(
                    "course_ends_soon",
                    format!("The course ends on {}.", end.iso()),
                );
            }
        }
    }
    out
}
