//! Calendar sync: keeps the hub's events for expiry, course end and
//! reordering in step with the cabinet.

mod client;
mod events;

use std::time::Duration;

use chassis::Error;
use chassis::storage::AppDb;
use chrono::{Datelike, Utc};

use crate::config::ReminderConfig;
use crate::domain::date::Date;
use crate::store::{agenda_sent, medicines};
use client::Hub;
pub use client::HubConfig;
use events::{Wanted, wanted_events};

const PASS_EVERY: Duration = Duration::from_secs(15 * 60);

/// What a pass has to do, decided from the wanted set and the sent log.
#[derive(Debug, Default, PartialEq)]
pub struct Plan {
    pub upsert: Vec<Wanted>,
    pub delete: Vec<String>,
}

/// Compare what is wanted with what the hub was last told.
pub fn plan(wanted: Vec<Wanted>, sent: &std::collections::HashMap<String, String>) -> Plan {
    let upsert = wanted
        .iter()
        .filter(|w| sent.get(&w.external_id) != Some(&w.fingerprint()))
        .cloned()
        .collect();
    let keep: std::collections::HashSet<&str> =
        wanted.iter().map(|w| w.external_id.as_str()).collect();
    let delete = sent
        .keys()
        .filter(|id| !keep.contains(id.as_str()))
        .cloned()
        .collect();
    Plan { upsert, delete }
}

pub fn spawn(db: AppDb, hub: HubConfig, reminders: ReminderConfig) {
    let hub = match Hub::new(hub) {
        Ok(h) => h,
        Err(e) => return eprintln!("agenda sync is off: {e}"),
    };
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(Duration::from_secs(20)).await;
            match pass(&db, &hub, reminders).await {
                Ok(()) => {}
                Err(e) if e.message.contains("not open yet") => continue,
                Err(e) => eprintln!("agenda sync: this pass failed: {e}"),
            }
            tokio::time::sleep(PASS_EVERY).await;
        }
    });
}

async fn pass(db: &AppDb, hub: &Hub, config: ReminderConfig) -> Result<(), Error> {
    let local = Utc::now().with_timezone(&config.timezone);
    let today = Date::parse(&format!(
        "{:04}-{:02}-{:02}",
        local.year(),
        local.month(),
        local.day()
    ))?;
    let (meds, sent) = db
        .read(move |conn| Ok((medicines::list(conn, today)?, agenda_sent::all(conn)?)))
        .await?;
    let todo = plan(wanted_events(&meds, today), &sent);
    for event in todo.upsert {
        hub.upsert(&event).await?;
        let (id, print) = (event.external_id.clone(), event.fingerprint());
        db.write(move |tx| agenda_sent::remember(tx, &id, &print))
            .await?;
    }
    for id in todo.delete {
        hub.delete(&id).await?;
        db.write(move |tx| agenda_sent::forget(tx, &id)).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn wanted(id: &str, start: &str) -> Wanted {
        Wanted {
            external_id: id.into(),
            title: "T".into(),
            start: start.into(),
            all_day: true,
            description: String::new(),
        }
    }

    #[test]
    fn only_changed_events_are_sent_and_stale_ones_deleted() {
        let same = wanted("a", "2026-11-01");
        let moved = wanted("b", "2026-11-05");
        let new = wanted("c", "2026-11-09");
        let mut sent = HashMap::new();
        sent.insert("a".to_string(), same.fingerprint());
        sent.insert("b".to_string(), wanted("b", "2026-11-04").fingerprint());
        sent.insert("gone".to_string(), "x".to_string());
        let p = plan(vec![same, moved.clone(), new.clone()], &sent);
        assert_eq!(p.upsert, vec![moved, new]);
        assert_eq!(p.delete, vec!["gone".to_string()]);
    }
}
