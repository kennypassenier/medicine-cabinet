//! What needs a person's attention in the cabinet: stock, expiry, courses.

use serde::Serialize;

use crate::domain::date::Date;
use crate::domain::medicine::{Freshness, StockLevel};
use crate::store::medicines::Medicine;

#[derive(Debug, Serialize)]
pub struct Attention {
    #[serde(skip)]
    pub medicine_id: i64,
    /// The medicine's public identifier, serialized as `medicine_id`.
    #[serde(rename = "medicine_id")]
    pub medicine_uid: String,
    pub medicine_name: String,
    /// `out_of_stock`, `low_stock`, `expired`, `expiring_soon`, `course_ends_soon`.
    pub kind: &'static str,
    pub detail: String,
}

pub fn attention_for(all: &[Medicine], today: Date) -> Vec<Attention> {
    let mut out = Vec::new();
    for m in all.iter().filter(|m| m.active) {
        let mut push = |kind, detail: String| {
            out.push(Attention {
                medicine_id: m.id,
                medicine_uid: m.uid.clone(),
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

#[cfg(test)]
mod tests;
