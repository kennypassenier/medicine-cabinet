//! Shared fixtures for unit tests: one fully filled `Medicine` to adjust.

use crate::domain::medicine::{Freshness, StockLevel};
use crate::domain::schedule::SlotInput;
use crate::store::medicines::Medicine;

/// An active, scheduled, in-stock medicine with one daily 08:00 slot.
pub fn medicine(id: i64, name: &str) -> Medicine {
    Medicine {
        id,
        name: name.into(),
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
