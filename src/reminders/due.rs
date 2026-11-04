//! Which reminders are due right now (pure: no clock, no database).

use std::collections::HashSet;

use crate::attention::Attention;
use crate::domain::plan::{PlannedDose, SlotState};
use crate::domain::units::amount;
use crate::store::reminder_log::SentKey;

/// One notification to send.
#[derive(Debug, PartialEq, Eq)]
pub struct Reminder {
    pub key: SentKey,
    /// The notification event kind (`dose.due`, `dose.overdue`, `cabinet.attention`).
    pub kind: &'static str,
    pub text: String,
}

/// An overdue dose is flagged once, and only while it is this recent.
const OVERDUE_LIMIT_MINUTES: i64 = 180;

/// Dose reminders for `date` given the day's plan.
pub fn dose_reminders(date: &str, doses: &[PlannedDose], sent: &HashSet<SentKey>) -> Vec<Reminder> {
    let mut out = Vec::new();
    for d in doses {
        let (kind, tag) = match d.state {
            SlotState::Due => ("dose.due", "due"),
            SlotState::Overdue if d.minutes_until >= -OVERDUE_LIMIT_MINUTES => {
                ("dose.overdue", "overdue")
            }
            _ => continue,
        };
        let key = (d.medicine_id, format!("{date}T{}", d.time), tag.to_string());
        if sent.contains(&key) {
            continue;
        }
        let text = match d.state {
            SlotState::Due => format!(
                "Time for {} ({}), planned at {}.",
                d.medicine_name,
                amount(d.dose, &d.unit),
                d.time
            ),
            _ => format!(
                "{} was planned at {} and is not logged yet.",
                d.medicine_name, d.time
            ),
        };
        out.push(Reminder { key, kind, text });
    }
    out
}

/// The daily digest: one reminder per attention item, once per day.
pub fn attention_reminders(
    date: &str,
    items: &[Attention],
    sent: &HashSet<SentKey>,
) -> Vec<Reminder> {
    items
        .iter()
        .map(|a| ((a.medicine_id, date.to_string(), a.kind.to_string()), a))
        .filter(|(key, _)| !sent.contains(key))
        .map(|(key, a)| Reminder {
            key,
            kind: "cabinet.attention",
            text: format!("{}: {}", a.medicine_name, a.detail),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dose(state: SlotState, minutes_until: i64) -> PlannedDose {
        PlannedDose {
            medicine_id: 1,
            medicine_uid: "uid-m1".into(),
            medicine_name: "Vitamin D".into(),
            time: "08:00".into(),
            dose: 1.0,
            unit: "capsules".into(),
            state,
            intake_id: None,
            minutes_until,
        }
    }

    #[test]
    fn a_due_dose_is_reminded_once() {
        let doses = [dose(SlotState::Due, -5)];
        let first = dose_reminders("2026-10-30", &doses, &HashSet::new());
        assert_eq!(first.len(), 1);
        assert_eq!(first[0].kind, "dose.due");
        let sent: HashSet<SentKey> = first.into_iter().map(|r| r.key).collect();
        assert!(dose_reminders("2026-10-30", &doses, &sent).is_empty());
    }

    #[test]
    fn upcoming_taken_and_ancient_doses_stay_quiet() {
        let doses = [
            dose(SlotState::Upcoming, 30),
            dose(SlotState::Taken, -30),
            dose(SlotState::Skipped, -30),
            dose(SlotState::Overdue, -600),
        ];
        assert!(dose_reminders("2026-10-30", &doses, &HashSet::new()).is_empty());
    }

    #[test]
    fn a_recently_overdue_dose_gets_one_nudge() {
        let r = dose_reminders(
            "2026-10-30",
            &[dose(SlotState::Overdue, -90)],
            &HashSet::new(),
        );
        assert_eq!(r[0].kind, "dose.overdue");
    }

    #[test]
    fn the_digest_skips_what_was_sent() {
        let item = Attention {
            medicine_id: 2,
            medicine_uid: "uid-m2".into(),
            medicine_name: "X".into(),
            kind: "low_stock",
            detail: "Low.".into(),
        };
        let mut sent = HashSet::new();
        assert_eq!(
            attention_reminders("2026-10-30", std::slice::from_ref(&item), &sent).len(),
            1
        );
        sent.insert((2, "2026-10-30".to_string(), "low_stock".to_string()));
        assert!(attention_reminders("2026-10-30", &[item], &sent).is_empty());
    }
}

#[cfg(test)]
mod edge_tests;
