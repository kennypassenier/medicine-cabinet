//! Turning schedule slots and logged intakes into a day plan.

use serde::Serialize;

/// One scheduled dose of one medicine on one day, with what is known of it.
#[derive(Debug, Clone)]
pub struct PlanSlot {
    pub medicine_id: i64,
    /// The medicine's public identifier.
    pub medicine_uid: String,
    pub medicine_name: String,
    /// Minutes since midnight.
    pub minute: u32,
    pub time: String,
    pub dose: f64,
    /// What stock is counted in ("capsules"); may be empty.
    pub unit: String,
    /// `(intake id, status)` when this slot has been answered.
    pub answered: Option<(i64, String)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SlotState {
    Upcoming,
    Due,
    Overdue,
    Taken,
    Skipped,
}

/// A dose is "due" from its time until this many minutes later; after that
/// it counts as overdue.
pub const DUE_WINDOW_MINUTES: i64 = 60;

#[derive(Debug, Serialize)]
pub struct PlannedDose {
    #[serde(skip)]
    pub medicine_id: i64,
    /// The medicine's public identifier, serialized as `medicine_id`.
    #[serde(rename = "medicine_id")]
    pub medicine_uid: String,
    pub medicine_name: String,
    pub time: String,
    pub dose: f64,
    pub unit: String,
    pub state: SlotState,
    pub intake_id: Option<i64>,
    /// Minutes until the dose (negative once it is late).
    pub minutes_until: i64,
}

pub fn state_of(slot: &PlanSlot, now_minute: u32) -> SlotState {
    match slot.answered.as_ref().map(|(_, s)| s.as_str()) {
        Some("taken") => return SlotState::Taken,
        Some(_) => return SlotState::Skipped,
        None => {}
    }
    let late = i64::from(now_minute) - i64::from(slot.minute);
    if late < 0 {
        SlotState::Upcoming
    } else if late <= DUE_WINDOW_MINUTES {
        SlotState::Due
    } else {
        SlotState::Overdue
    }
}

/// The day's doses in time order.
pub fn plan(mut slots: Vec<PlanSlot>, now_minute: u32) -> Vec<PlannedDose> {
    slots.sort_by_key(|s| (s.minute, s.medicine_name.clone()));
    slots
        .into_iter()
        .map(|s| PlannedDose {
            state: state_of(&s, now_minute),
            intake_id: s.answered.as_ref().map(|(id, _)| *id),
            minutes_until: i64::from(s.minute) - i64::from(now_minute),
            medicine_id: s.medicine_id,
            medicine_uid: s.medicine_uid,
            medicine_name: s.medicine_name,
            time: s.time,
            dose: s.dose,
            unit: s.unit,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn slot(minute: u32, answered: Option<&str>) -> PlanSlot {
        PlanSlot {
            medicine_id: 1,
            medicine_uid: "uid-1".into(),
            medicine_name: "Vitamin D".into(),
            minute,
            time: format!("{:02}:{:02}", minute / 60, minute % 60),
            dose: 1.0,
            unit: "capsules".into(),
            answered: answered.map(|s| (7, s.to_string())),
        }
    }

    #[test]
    fn states_follow_the_clock() {
        assert_eq!(state_of(&slot(600, None), 540), SlotState::Upcoming);
        assert_eq!(state_of(&slot(600, None), 630), SlotState::Due);
        assert_eq!(state_of(&slot(600, None), 700), SlotState::Overdue);
    }

    #[test]
    fn an_answer_beats_the_clock() {
        assert_eq!(state_of(&slot(600, Some("taken")), 900), SlotState::Taken);
        assert_eq!(
            state_of(&slot(600, Some("skipped")), 540),
            SlotState::Skipped
        );
    }

    #[test]
    fn plan_is_sorted_by_time() {
        let p = plan(vec![slot(900, None), slot(480, None)], 600);
        assert_eq!(p[0].time, "08:00");
        assert_eq!(p[1].time, "15:00");
    }
}

#[cfg(test)]
mod edge_tests;
