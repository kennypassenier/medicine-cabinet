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

const URL_VAR: &str = "MEDICINE_CABINET_ALMANAC_URL";
const TOKEN_VAR: &str = "MEDICINE_CABINET_ALMANAC_TOKEN";
const SOURCE_VAR: &str = "MEDICINE_CABINET_ALMANAC_SOURCE";

/// The calendar hub, optional: without an address the sync stays off. An
/// address without a token (or the reverse), or an address that is not an
/// http(s) URL, is a mistake worth stopping the start for, not a sync that
/// silently never runs.
pub fn hub_from_env() -> Result<Option<crate::agenda::HubConfig>, Error> {
    let get = |name: &str| {
        std::env::var(name)
            .ok()
            .map(|v| v.trim().to_string())
            .filter(|v| !v.is_empty())
    };
    parse_hub(get(URL_VAR), get(TOKEN_VAR), get(SOURCE_VAR))
}

fn parse_hub(
    url: Option<String>,
    token: Option<String>,
    source: Option<String>,
) -> Result<Option<crate::agenda::HubConfig>, Error> {
    let (base, token) = match (url, token) {
        (None, None) => return Ok(None),
        (Some(base), Some(token)) => (base, token),
        (Some(_), None) => {
            return Err(Error::config(
                format!("{URL_VAR} is set but {TOKEN_VAR} is not"),
                format!(
                    "set {TOKEN_VAR} to the source's token, or unset {URL_VAR} to turn the calendar sync off"
                ),
            ));
        }
        (None, Some(_)) => {
            return Err(Error::config(
                format!("{TOKEN_VAR} is set but {URL_VAR} is not"),
                format!(
                    "set {URL_VAR} to the hub's address, or unset {TOKEN_VAR} to turn the calendar sync off"
                ),
            ));
        }
    };
    if !(base.starts_with("http://") || base.starts_with("https://")) {
        return Err(Error::config(
            format!("{URL_VAR} must start with http:// or https://"),
            "use an address such as http://10.10.10.12:8080",
        ));
    }
    let source_id = source.unwrap_or_else(|| "medicine-cabinet".into());
    let safe = source_id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    if !safe {
        return Err(Error::config(
            format!("{SOURCE_VAR} may only hold letters, digits, '-', '_' and '.'"),
            "use the source id Almanac shows for this app",
        ));
    }
    Ok(Some(crate::agenda::HubConfig {
        base_url: base.trim_end_matches('/').to_string(),
        source_id,
        token,
    }))
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
    fn the_hub_is_all_or_nothing() {
        assert!(parse_hub(None, None, None).unwrap().is_none());
        let on = parse_hub(Some("http://hub:1/".into()), Some("t".into()), None).unwrap();
        assert_eq!(on.unwrap().base_url, "http://hub:1");
        for (u, t, s) in [
            (Some("http://hub"), None, None),
            (None, Some("t"), None),
            (Some("hub:80"), Some("t"), None),
            (Some("http://hub"), Some("t"), Some("a/b")),
        ] {
            let err = parse_hub(u.map(Into::into), t.map(Into::into), s.map(Into::into));
            let err = err.unwrap_err();
            assert!(!err.remedy.is_empty());
        }
    }

    #[test]
    fn refuses_nonsense_with_a_remedy() {
        assert!(ReminderConfig::parse(Some("Mars/Base".into()), None).is_err());
        assert!(ReminderConfig::parse(None, Some("25".into())).is_err());
    }
}
