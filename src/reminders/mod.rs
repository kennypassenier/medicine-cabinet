//! Reminders: a background loop that turns the day's plan into
//! notifications (through the kit's notifier, so they reach kyu).

mod collect;
mod due;

use std::time::Duration;

use chassis::Error;
use chassis::shell::notify::Notifier;
use chassis::storage::AppDb;

use crate::clock::now_in;
use crate::config::ReminderConfig;
use crate::store::reminder_log;
use collect::collect;

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

/// One pass: find what is due, hand it to the notifier, and only then mark
/// it as sent.
///
/// A crash between emit and mark re-sends that reminder on the next tick (it
/// may arrive twice) but never loses one. The kit's `Notifier::emit` is fire
/// and forget and reports no delivery result, so "sent" here means "emit
/// returned"; whether kyu received it is not known to this loop.
async fn tick(db: &AppDb, notifier: &Notifier, config: ReminderConfig) -> Result<(), Error> {
    let now = now_in(config.timezone);
    let digest_due = now.hour >= config.digest_hour;
    let reminders = db
        .write(move |tx| collect(tx, now.date, now.minute, digest_due))
        .await?;
    for r in reminders {
        notifier.emit(r.kind, env!("CARGO_PKG_VERSION"), r.text);
        let key = r.key;
        db.write(move |tx| reminder_log::mark(tx, &key).map(|_| ()))
            .await?;
    }
    Ok(())
}
