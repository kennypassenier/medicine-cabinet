//! Reading step of a tick: what is due now. It marks nothing as sent; the
//! caller marks a reminder only after the notifier has taken it.

use std::collections::HashSet;

use chassis::Error;
use chassis::storage::rusqlite::Transaction;

use super::due::{Reminder, attention_reminders, dose_reminders};
use crate::attention::attention_for;
use crate::dayplan::day_slots;
use crate::domain::date::Date;
use crate::domain::plan::plan;
use crate::store::intakes::Intake;
use crate::store::medicines::Medicine;
use crate::store::{intakes, medicines, reminder_log};

/// Minutes in a day: yesterday's plan is read with "now" shifted by this much.
const DAY_MINUTES: u32 = 24 * 60;

/// Everything due now and not yet in the sent ledger. Also housekeeping
/// (prune old log rows, purge deleted medicines).
pub fn collect(
    tx: &Transaction,
    today: Date,
    minute: u32,
    digest_due: bool,
) -> Result<Vec<Reminder>, Error> {
    let (day, yesterday) = (today.iso(), today.plus_days(-1));
    let all = medicines::list(tx, today)?;
    let logged_today = intakes::list(tx, &day, 500)?;
    let sent = reminder_log::sent_on(tx, &day)?;
    let doses = plan(day_slots(&all, &logged_today, today), minute);
    let mut due = dose_reminders(&day, &doses, &sent);
    // An intake for yesterday's slot can be logged after midnight, so the
    // answers for yesterday come from both days' intakes.
    let mut logged_yesterday = intakes::list(tx, &yesterday.iso(), 500)?;
    logged_yesterday.extend(logged_today);
    let sent_yesterday = reminder_log::sent_on(tx, &yesterday.iso())?;
    due.extend(carryover_reminders(
        &all,
        &logged_yesterday,
        yesterday,
        minute,
        &sent_yesterday,
    ));
    if digest_due {
        due.extend(attention_reminders(
            &day,
            &attention_for(&all, today),
            &sent,
        ));
    }
    reminder_log::prune(tx, &today.plus_days(-7).iso())?;
    medicines::purge_deleted(tx)?;
    Ok(due)
}

/// Overdue nudges for yesterday's doses that fall past midnight: the day plan
/// is read as if it were still yesterday, `minute` minutes into today being
/// `minute + 1440` minutes into yesterday. Only `dose.overdue` carries over
/// (the 180-minute limit is applied by `dose_reminders`); the sent key uses
/// yesterday's date, so it is sent once.
pub fn carryover_reminders(
    all: &[Medicine],
    logged: &[Intake],
    yesterday: Date,
    minute: u32,
    sent: &HashSet<reminder_log::SentKey>,
) -> Vec<Reminder> {
    let doses = plan(day_slots(all, logged, yesterday), minute + DAY_MINUTES);
    let mut out = dose_reminders(&yesterday.iso(), &doses, sent);
    out.retain(|r| r.kind == "dose.overdue");
    out
}

#[cfg(test)]
mod tests;
