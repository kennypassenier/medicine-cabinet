//! Reminders: a background loop that turns the day's plan into
//! notifications (through the kit's notifier, so they reach kyu).

mod due;

use std::time::Duration;

use chassis::Error;
use chassis::shell::notify::Notifier;
use chassis::storage::AppDb;
use chrono::{Datelike, Timelike, Utc};

use crate::attention::attention_for;
use crate::config::ReminderConfig;
use crate::dayplan::day_slots;
use crate::domain::date::Date;
use crate::domain::plan::plan;
use crate::store::{intakes, medicines, reminder_log};
use due::{Reminder, attention_reminders, dose_reminders};

const TICK: Duration = Duration::from_secs(30);

/// Start the loop. It waits for the database to open, then ticks forever.
pub fn spawn(db: AppDb, notifier: Notifier, config: ReminderConfig) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(TICK).await;
            match tick(&db, &notifier, config).await {
                Ok(()) => {}
                Err(e) if e.message.contains("not open yet") => {}
                Err(e) => eprintln!("reminders: this tick failed: {e}"),
            }
        }
    });
}

/// One pass: find what is due, mark it sent, then notify.
async fn tick(db: &AppDb, notifier: &Notifier, config: ReminderConfig) -> Result<(), Error> {
    let local = Utc::now().with_timezone(&config.timezone);
    let today = Date::parse(&format!(
        "{:04}-{:02}-{:02}",
        local.year(),
        local.month(),
        local.day()
    ))?;
    let minute = local.hour() * 60 + local.minute();
    let digest_due = local.hour() >= config.digest_hour;
    let reminders = db
        .write(move |tx| collect(tx, today, minute, digest_due))
        .await?;
    for r in reminders {
        notifier.emit(r.kind, env!("CARGO_PKG_VERSION"), r.text);
    }
    Ok(())
}

/// Everything due now, already marked as sent inside this transaction.
fn collect(
    tx: &chassis::storage::rusqlite::Transaction,
    today: Date,
    minute: u32,
    digest_due: bool,
) -> Result<Vec<Reminder>, Error> {
    let day = today.iso();
    let all = medicines::list(tx, today)?;
    let logged = intakes::list(tx, &day, 500)?;
    let sent = reminder_log::sent_on(tx, &day)?;
    let doses = plan(day_slots(&all, &logged, today), minute);
    let mut due = dose_reminders(&day, &doses, &sent);
    if digest_due {
        due.extend(attention_reminders(
            &day,
            &attention_for(&all, today),
            &sent,
        ));
    }
    let mut fresh = Vec::with_capacity(due.len());
    for r in due {
        // Another tick may have marked it first; then it is not ours to send.
        if reminder_log::mark(tx, &r.key)? {
            fresh.push(r);
        }
    }
    reminder_log::prune(tx, &today.plus_days(-7).iso())?;
    medicines::purge_deleted(tx)?;
    Ok(fresh)
}
