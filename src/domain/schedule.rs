//! Repeating schedule slots and the as-needed dosing guard.

use chassis::Error;
use serde::{Deserialize, Serialize};

use super::date::{Date, parse_minutes};

/// One repeating slot: a time of day on some weekdays.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct SlotInput {
    /// `HH:MM`.
    pub time: String,
    /// Bit 0 = Monday … bit 6 = Sunday; 127 = every day.
    #[serde(default = "every_day")]
    pub days: u8,
    #[serde(default = "one")]
    pub dose: f64,
}

fn every_day() -> u8 {
    0b111_1111
}

fn one() -> f64 {
    1.0
}

impl SlotInput {
    pub fn validated(self) -> Result<Self, Error> {
        parse_minutes(&self.time)?;
        if self.days == 0 || self.days > 0b111_1111 {
            return Err(Error::invalid(
                "a schedule slot needs at least one weekday",
                "pick one or more days",
            ));
        }
        if self.dose <= 0.0 {
            return Err(Error::invalid(
                "a dose must be more than zero",
                "enter the amount taken at once",
            ));
        }
        Ok(self)
    }

    pub fn runs_on(&self, date: Date) -> bool {
        self.days & (1 << date.weekday()) != 0
    }
}

/// A past intake, reduced to what the dosing guard needs.
#[derive(Debug, Clone, Copy)]
pub struct PastDose {
    /// Minutes since the start of the day the guard runs on; may be negative
    /// for earlier days.
    pub minutes_ago: i64,
}

/// Verdict for taking an as-needed dose right now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DoseVerdict {
    pub allowed: bool,
    /// Plain-language reason when not allowed.
    pub reason: Option<String>,
    /// Minutes until the next dose is allowed.
    pub wait_minutes: Option<u32>,
}

/// Apply the minimum gap and the 24-hour maximum.
pub fn check_dose(
    past: &[PastDose],
    min_gap_minutes: Option<u32>,
    max_per_day: Option<u32>,
) -> DoseVerdict {
    let ok = DoseVerdict {
        allowed: true,
        reason: None,
        wait_minutes: None,
    };
    if let (Some(gap), Some(last)) = (min_gap_minutes, past.iter().map(|p| p.minutes_ago).min()) {
        let wait = i64::from(gap) - last;
        if wait > 0 {
            return DoseVerdict {
                allowed: false,
                reason: Some(format!(
                    "The last dose was {last} minutes ago; wait {wait} more."
                )),
                wait_minutes: Some(wait as u32),
            };
        }
    }
    if let Some(max) = max_per_day {
        let last_day = past.iter().filter(|p| p.minutes_ago < 24 * 60).count() as u32;
        if last_day >= max {
            let oldest = past
                .iter()
                .filter(|p| p.minutes_ago < 24 * 60)
                .map(|p| p.minutes_ago)
                .max();
            let wait = oldest.map(|o| (24 * 60 - o) as u32);
            return DoseVerdict {
                allowed: false,
                reason: Some(format!(
                    "{last_day} doses in the last 24 hours is the maximum ({max})."
                )),
                wait_minutes: wait,
            };
        }
    }
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_runs_on_selected_weekdays() {
        let monday_only = SlotInput {
            time: "08:00".into(),
            days: 0b1,
            dose: 1.0,
        };
        assert!(monday_only.runs_on(Date::parse("2026-10-26").unwrap()));
        assert!(!monday_only.runs_on(Date::parse("2026-10-27").unwrap()));
    }

    #[test]
    fn slot_validation() {
        assert!(
            SlotInput {
                time: "8am".into(),
                days: 1,
                dose: 1.0
            }
            .validated()
            .is_err()
        );
        assert!(
            SlotInput {
                time: "08:00".into(),
                days: 0,
                dose: 1.0
            }
            .validated()
            .is_err()
        );
        assert!(
            SlotInput {
                time: "08:00".into(),
                days: 3,
                dose: 0.0
            }
            .validated()
            .is_err()
        );
    }

    #[test]
    fn minimum_gap_blocks_and_reports_the_wait() {
        let v = check_dose(&[PastDose { minutes_ago: 90 }], Some(240), None);
        assert!(!v.allowed);
        assert_eq!(v.wait_minutes, Some(150));
    }

    #[test]
    fn daily_maximum_blocks() {
        let past = [PastDose { minutes_ago: 60 }, PastDose { minutes_ago: 600 }];
        assert!(!check_dose(&past, None, Some(2)).allowed);
        assert!(check_dose(&past, None, Some(3)).allowed);
    }

    #[test]
    fn nothing_taken_is_always_allowed() {
        assert!(check_dose(&[], Some(240), Some(1)).allowed);
    }
}

#[cfg(test)]
mod edge_tests;
