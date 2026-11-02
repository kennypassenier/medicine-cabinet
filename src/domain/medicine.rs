//! A medicine as the user enters it, and the judgements derived from it.

use chassis::Error;
use serde::{Deserialize, Serialize};

use super::date::Date;
use super::limits;

/// What the user types in (create and update share one shape).
#[derive(Debug, Clone, Deserialize)]
pub struct MedicineInput {
    pub name: String,
    #[serde(default)]
    pub purpose: String,
    /// One of the managed forms (tablet, syrup, ...).
    pub form_id: Option<i64>,
    #[serde(default)]
    pub strength: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default = "yes")]
    pub active: bool,
    #[serde(default)]
    pub as_needed: bool,
    /// As-needed medicines: least minutes between two doses.
    pub min_gap_minutes: Option<u32>,
    /// Most doses in 24 hours.
    pub max_per_day: Option<u32>,
    #[serde(default)]
    pub stock: f64,
    #[serde(default)]
    pub stock_unit: String,
    pub low_stock_at: Option<f64>,
    pub expires_on: Option<String>,
    pub opened_on: Option<String>,
    /// Days the medicine stays good after opening.
    pub shelf_life_days: Option<u32>,
    /// A course (an antibiotic) stops on this date.
    pub course_ends_on: Option<String>,
    #[serde(default)]
    pub symptoms: Vec<String>,
}

fn yes() -> bool {
    true
}

impl MedicineInput {
    /// Trim, check and normalise; the store only ever sees a valid input.
    pub fn validated(mut self) -> Result<Self, Error> {
        self.name = limits::text("the name", &self.name, limits::NAME_MAX, true)?;
        if self.name.is_empty() {
            return Err(Error::invalid(
                "a medicine needs a name",
                "fill in the name field",
            ));
        }
        self.purpose = limits::text("the purpose", &self.purpose, limits::TEXT_MAX, false)?;
        self.strength = limits::text("the strength", &self.strength, limits::UNIT_MAX, true)?;
        self.notes = limits::text("the notes", &self.notes, limits::TEXT_MAX, false)?;
        self.stock_unit = limits::text("the stock unit", &self.stock_unit, limits::UNIT_MAX, true)?;
        self.stock = limits::amount("stock", self.stock, false)?;
        if let Some(v) = self.low_stock_at {
            self.low_stock_at = Some(limits::amount("the low-stock level", v, false)?);
        }
        self.min_gap_minutes = self
            .min_gap_minutes
            .map(|v| limits::whole("the minimum gap in minutes", v, 1, 7 * 24 * 60))
            .transpose()?;
        self.max_per_day = self
            .max_per_day
            .map(|v| limits::whole("the maximum per day", v, 1, 1000))
            .transpose()?;
        self.shelf_life_days = self
            .shelf_life_days
            .map(|v| limits::whole("the shelf life in days", v, 1, 36_500))
            .transpose()?;
        for date in [
            &mut self.expires_on,
            &mut self.opened_on,
            &mut self.course_ends_on,
        ] {
            *date = match date.take().filter(|d| !d.trim().is_empty()) {
                Some(d) => Some(Date::parse(d.trim())?.iso()),
                None => None,
            };
        }
        limits::list_len("the symptom list", self.symptoms.len())?;
        self.symptoms = clean_symptoms(&self.symptoms)?;
        Ok(self)
    }
}

/// Symptom names: trimmed, lowercased, de-duplicated, order kept.
pub fn clean_symptoms(raw: &[String]) -> Result<Vec<String>, Error> {
    let mut out: Vec<String> = Vec::new();
    for name in raw {
        let name = limits::text("a symptom name", name, limits::NAME_MAX, true)?.to_lowercase();
        if !name.is_empty() && !out.contains(&name) {
            out.push(name);
        }
    }
    Ok(out)
}

/// How soon a date runs out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Freshness {
    Fresh,
    /// Within 30 days.
    ExpiringSoon,
    Expired,
}

const SOON_DAYS: i64 = 30;

/// The earlier of the printed expiry and "opened + shelf life".
pub fn effective_expiry(
    expires_on: Option<&str>,
    opened_on: Option<&str>,
    shelf_life_days: Option<u32>,
) -> Option<Date> {
    let printed = expires_on.and_then(|d| Date::parse(d).ok());
    let after_opening = match (opened_on, shelf_life_days) {
        (Some(o), Some(days)) => Date::parse(o).ok().map(|o| o.plus_days(i64::from(days))),
        _ => None,
    };
    match (printed, after_opening) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    }
}

pub fn freshness(expiry: Option<Date>, today: Date) -> Freshness {
    match expiry.map(|e| today.days_until(e)) {
        Some(left) if left < 0 => Freshness::Expired,
        Some(left) if left <= SOON_DAYS => Freshness::ExpiringSoon,
        _ => Freshness::Fresh,
    }
}

/// Stock health: `Out` at zero, `Low` at or under the threshold.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum StockLevel {
    Ok,
    Low,
    Out,
}

pub fn stock_level(stock: f64, low_at: Option<f64>) -> StockLevel {
    if stock <= 0.0 {
        StockLevel::Out
    } else if low_at.is_some_and(|t| stock <= t) {
        StockLevel::Low
    } else {
        StockLevel::Ok
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Date {
        Date::parse(s).unwrap()
    }

    #[test]
    fn the_earlier_of_printed_and_opened_expiry_wins() {
        let e = effective_expiry(Some("2027-01-01"), Some("2026-10-01"), Some(30));
        assert_eq!(e, Some(d("2026-10-31")));
    }

    #[test]
    fn freshness_flags_soon_and_past() {
        let today = d("2026-10-30");
        assert_eq!(freshness(Some(d("2026-10-29")), today), Freshness::Expired);
        assert_eq!(
            freshness(Some(d("2026-11-20")), today),
            Freshness::ExpiringSoon
        );
        assert_eq!(freshness(Some(d("2027-06-01")), today), Freshness::Fresh);
        assert_eq!(freshness(None, today), Freshness::Fresh);
    }

    #[test]
    fn stock_levels() {
        assert_eq!(stock_level(0.0, Some(5.0)), StockLevel::Out);
        assert_eq!(stock_level(4.0, Some(5.0)), StockLevel::Low);
        assert_eq!(stock_level(9.0, Some(5.0)), StockLevel::Ok);
        assert_eq!(stock_level(9.0, None), StockLevel::Ok);
    }

    #[test]
    fn symptoms_are_normalised() {
        let raw = vec![
            " Headache ".into(),
            "headache".into(),
            "".into(),
            "Fever".into(),
        ];
        assert_eq!(clean_symptoms(&raw).unwrap(), vec!["headache", "fever"]);
        assert!(clean_symptoms(&["x".repeat(500)]).is_err());
    }
}

#[cfg(test)]
mod edge_tests;
#[cfg(test)]
mod validation_tests;
