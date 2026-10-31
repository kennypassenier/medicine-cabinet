//! The app's own settings, read from the environment once at start.
//!
//! The kit owns the standard knobs (listen address, state dir, secrets);
//! these two belong to the reminders. Both come from the service's
//! environment file, never from code.

use chassis::Error;
use chrono_tz::Tz;

/// Settings for the reminder scheduler.
#[derive(Debug, Clone, Copy)]
pub struct ReminderConfig {
    /// The zone schedules are written in ("08:00" means 08:00 here).
    pub timezone: Tz,
    /// Local hour from which the daily stock/expiry digest is sent.
    pub digest_hour: u32,
}

const ZONE_VAR: &str = "MEDICINE_CABINET_TIMEZONE";
const HOUR_VAR: &str = "MEDICINE_CABINET_DIGEST_HOUR";

impl ReminderConfig {
    pub fn from_env() -> Result<Self, Error> {
        Self::parse(std::env::var(ZONE_VAR).ok(), std::env::var(HOUR_VAR).ok())
    }

    fn parse(zone: Option<String>, hour: Option<String>) -> Result<Self, Error> {
        let timezone = match zone.as_deref().map(str::trim).filter(|z| !z.is_empty()) {
            None => Tz::UTC,
            Some(name) => name.parse::<Tz>().map_err(|_| {
                Error::config(
                    format!("{ZONE_VAR} is '{name}', which is not a time zone"),
                    "use an IANA name such as Europe/Brussels",
                )
            })?,
        };
        let digest_hour = match hour.as_deref().map(str::trim).filter(|h| !h.is_empty()) {
            None => 9,
            Some(text) => text
                .parse::<u32>()
                .ok()
                .filter(|h| *h < 24)
                .ok_or_else(|| {
                    Error::config(
                        format!("{HOUR_VAR} is '{text}', which is not an hour"),
                        "use a whole number from 0 to 23",
                    )
                })?,
        };
        Ok(Self {
            timezone,
            digest_hour,
        })
    }
}

/// The calendar hub, optional: without an address the sync stays off.
pub fn hub_from_env() -> Option<crate::agenda::HubConfig> {
    let base = std::env::var("MEDICINE_CABINET_ALMANAC_URL").ok()?;
    let token = std::env::var("MEDICINE_CABINET_ALMANAC_TOKEN").ok()?;
    let source_id = std::env::var("MEDICINE_CABINET_ALMANAC_SOURCE")
        .unwrap_or_else(|_| "medicine-cabinet".into());
    let base_url = base.trim().trim_end_matches('/').to_string();
    (!base_url.is_empty() && !token.trim().is_empty()).then(|| crate::agenda::HubConfig {
        base_url,
        source_id,
        token: token.trim().to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_utc_and_nine() {
        let c = ReminderConfig::parse(None, None).unwrap();
        assert_eq!((c.timezone, c.digest_hour), (Tz::UTC, 9));
    }

    #[test]
    fn reads_a_zone_and_an_hour() {
        let c = ReminderConfig::parse(Some("Europe/Brussels".into()), Some("7".into())).unwrap();
        assert_eq!((c.timezone, c.digest_hour), (Tz::Europe__Brussels, 7));
    }

    #[test]
    fn refuses_nonsense_with_a_remedy() {
        assert!(ReminderConfig::parse(Some("Mars/Base".into()), None).is_err());
        assert!(ReminderConfig::parse(None, Some("25".into())).is_err());
    }
}
