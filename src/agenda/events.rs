//! The calendar events the cabinet wants to exist (pure: no I/O).
//!
//! Only dates worth planning around go on the calendar: when a medicine
//! expires, when a course ends, and when to reorder before stock runs out.
//! Individual doses stay out: the reminders cover those.

use serde::Serialize;

use crate::domain::date::Date;
use crate::store::medicines::Medicine;

/// Days before the predicted run-out on which the reorder event sits.
pub const REORDER_LEAD_DAYS: i64 = 7;
/// Nothing further ahead than this is planned.
const HORIZON_DAYS: i64 = 120;
/// Longest run-out prediction, in days (about 270 years).
const MAX_RUN_OUT_DAYS: f64 = 100_000.0;

/// One all-day event, in the shape the hub's ingest takes.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Wanted {
    pub external_id: String,
    pub title: String,
    pub start: String,
    pub all_day: bool,
    pub description: String,
}

impl Wanted {
    /// Changes whenever anything the hub shows changes.
    pub fn fingerprint(&self) -> String {
        format!("{}|{}|{}", self.title, self.start, self.description)
    }
}

fn all_day(external_id: String, title: String, date: Date, description: String) -> Wanted {
    Wanted {
        external_id,
        title,
        start: date.iso(),
        all_day: true,
        description,
    }
}

/// Doses per day on average over a week, from the schedule slots.
fn daily_use(m: &Medicine) -> f64 {
    m.slots
        .iter()
        .map(|s| s.dose * f64::from(s.days.count_ones()) / 7.0)
        .sum()
}

/// The day the stock is predicted to run out, for a scheduled medicine.
pub fn run_out_date(m: &Medicine, today: Date) -> Option<Date> {
    let per_day = daily_use(m);
    if !m.active || m.as_needed || per_day <= 0.0 || m.stock <= 0.0 {
        return None;
    }
    // Clamped: a tiny dose against a big stock must not overflow the date maths.
    let days = (m.stock / per_day).floor().min(MAX_RUN_OUT_DAYS);
    Some(today.plus_days(days as i64))
}

fn within_horizon(date: Date, today: Date) -> bool {
    (0..=HORIZON_DAYS).contains(&today.days_until(date))
}

pub fn wanted_events(medicines: &[Medicine], today: Date) -> Vec<Wanted> {
    let mut out = Vec::new();
    for m in medicines.iter().filter(|m| m.active) {
        let expiry = m
            .effective_expiry
            .as_deref()
            .and_then(|d| Date::parse(d).ok());
        if let Some(date) = expiry.filter(|d| within_horizon(*d, today)) {
            let id = format!("expiry:{}", m.id);
            out.push(all_day(
                id,
                format!("{} expires", m.name),
                date,
                expiry_note(m),
            ));
        }
        let course_end = m
            .course_ends_on
            .as_deref()
            .and_then(|d| Date::parse(d).ok());
        if let Some(date) = course_end.filter(|d| within_horizon(*d, today)) {
            let id = format!("course:{}", m.id);
            out.push(all_day(
                id,
                format!("Last day of {}", m.name),
                date,
                m.purpose.clone(),
            ));
        }
        if let Some(out_on) = run_out_date(m, today).filter(|d| within_horizon(*d, today)) {
            let date = out_on.plus_days(-REORDER_LEAD_DAYS).max(today);
            let id = format!("reorder:{}", m.id);
            let note = format!("{} runs out around {}.", m.name, out_on.iso());
            out.push(all_day(id, format!("Reorder {}", m.name), date, note));
        }
    }
    out
}

fn expiry_note(m: &Medicine) -> String {
    match &m.opened_on {
        Some(opened) => format!(
            "Opened on {opened}; good for {} days after opening.",
            m.shelf_life_days.unwrap_or(0)
        ),
        None => "Printed expiry date.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::medicine::{Freshness, StockLevel};
    use crate::domain::schedule::SlotInput;

    fn med() -> Medicine {
        Medicine {
            id: 3,
            rev: 1,
            name: "Vitamin D".into(),
            purpose: String::new(),
            form: String::new(),
            form_id: None,
            strength: String::new(),
            notes: String::new(),
            active: true,
            as_needed: false,
            min_gap_minutes: None,
            max_per_day: None,
            stock: 30.0,
            stock_unit: "capsules".into(),
            low_stock_at: None,
            expires_on: None,
            opened_on: None,
            shelf_life_days: None,
            course_ends_on: None,
            symptoms: vec![],
            slots: vec![SlotInput {
                time: "08:00".into(),
                days: 127,
                dose: 1.0,
            }],
            effective_expiry: None,
            freshness: Freshness::Fresh,
            stock_level: StockLevel::Ok,
            course_over: false,
        }
    }

    fn today() -> Date {
        Date::parse("2026-10-30").unwrap()
    }

    #[test]
    fn stock_prediction_follows_the_schedule() {
        // 30 capsules at one a day last 30 days.
        assert_eq!(run_out_date(&med(), today()).unwrap().iso(), "2026-11-29");
    }

    #[test]
    fn a_huge_stock_with_a_tiny_dose_cannot_overflow_the_date_maths() {
        let mut m = med();
        m.stock = 1_000_000.0;
        m.slots[0].dose = 1e-300;
        let far = run_out_date(&m, today()).unwrap();
        assert!(far.year < 3000);
        assert!(
            wanted_events(&[m], today()).is_empty(),
            "beyond the horizon"
        );
    }

    #[test]
    fn reorder_event_sits_a_week_before_the_run_out() {
        let events = wanted_events(&[med()], today());
        let reorder = events
            .iter()
            .find(|e| e.external_id.ends_with("reorder:3"))
            .unwrap();
        assert_eq!(reorder.start, "2026-11-22");
        assert!(reorder.all_day);
    }

    #[test]
    fn expiry_and_course_end_become_events_inside_the_horizon() {
        let mut m = med();
        m.effective_expiry = Some("2026-12-01".into());
        m.course_ends_on = Some("2026-11-02".into());
        let ids: Vec<_> = wanted_events(&[m], today())
            .into_iter()
            .map(|e| e.external_id)
            .collect();
        assert!(ids.contains(&"expiry:3".to_string()));
        assert!(ids.contains(&"course:3".to_string()));
    }

    #[test]
    fn switched_off_as_needed_and_empty_medicines_plan_nothing() {
        let mut off = med();
        off.active = false;
        let mut prn = med();
        prn.as_needed = true;
        prn.slots.clear();
        let mut empty = med();
        empty.stock = 0.0;
        assert!(wanted_events(&[off, prn], today()).is_empty());
        assert!(run_out_date(&empty, today()).is_none());
    }
}

#[cfg(test)]
mod edge_tests;
