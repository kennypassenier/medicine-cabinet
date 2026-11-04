//! Today's scheduled slots, paired with the intakes that answered them.

use crate::domain::date::{Date, parse_minutes};
use crate::domain::plan::PlanSlot;
use crate::store::intakes::Intake;
use crate::store::medicines::Medicine;

/// Scheduled slots that run today, paired with the intake that answered them.
pub fn day_slots(all: &[Medicine], logged: &[Intake], today: Date) -> Vec<PlanSlot> {
    let mut out = Vec::new();
    for m in all
        .iter()
        .filter(|m| m.active && !m.as_needed && !m.course_over)
    {
        for s in m.slots.iter().filter(|s| s.runs_on(today)) {
            let planned = format!("{}T{}", today.iso(), s.time);
            let answered = logged
                .iter()
                .find(|i| i.medicine_id == Some(m.id) && i.planned_for.as_deref() == Some(&planned))
                .map(|i| (i.id, i.status.clone()));
            out.push(PlanSlot {
                medicine_id: m.id,
                medicine_uid: m.uid.clone(),
                medicine_name: m.name.clone(),
                minute: parse_minutes(&s.time).unwrap_or(0),
                time: s.time.clone(),
                dose: s.dose,
                unit: m.stock_unit.clone(),
                answered,
            });
        }
    }
    out
}

#[cfg(test)]
mod tests;
