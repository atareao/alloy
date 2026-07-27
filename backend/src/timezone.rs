use chrono::{DateTime, FixedOffset, Utc};
use chrono_tz::Tz;
use std::sync::OnceLock;

static TIMEZONE: OnceLock<Tz> = OnceLock::new();

/// Initialize the configured timezone. Call once at startup.
/// Accepts IANA timezone names like "Europe/Madrid", "America/New_York", etc.
/// Defaults to UTC if not called or if the string is empty.
pub fn init(tz_str: &str) {
    let tz: Tz = if tz_str.is_empty() {
        chrono_tz::UTC
    } else {
        match tz_str.parse() {
            Ok(tz) => tz,
            Err(e) => {
                tracing::warn!("Invalid TIMEZONE '{}': {}. Falling back to UTC.", tz_str, e);
                chrono_tz::UTC
            }
        }
    };
    let _ = TIMEZONE.set(tz);
}

/// Returns the current UTC time as a `DateTime<FixedOffset>` in the configured timezone.
/// Falls back to UTC if the timezone hasn't been initialized.
pub fn now() -> DateTime<FixedOffset> {
    let tz = TIMEZONE.get().copied().unwrap_or(chrono_tz::UTC);
    Utc::now().with_timezone(&tz).fixed_offset()
}

/// Returns the timezone abbreviation (e.g., "CEST", "UTC", "EST") for the configured zone.
pub fn timezone_abbr() -> String {
    let tz = TIMEZONE.get().copied().unwrap_or(chrono_tz::UTC);
    Utc::now().with_timezone(&tz).format("%Z").to_string()
}

/// Formats the current time as `YYYY-MM-DDTHH:MM:SS±HH:MM` in the configured timezone.
pub fn now_formatted() -> String {
    now().format("%Y-%m-%dT%H:%M:%S%:z").to_string()
}

/// Formats the current time as `HH:MM:SS` in the configured timezone.
pub fn now_time_formatted() -> String {
    now().format("%H:%M:%S").to_string()
}

/// Compute the next scheduled time from a cron expression, in the configured timezone.
/// Returns `None` if the cron expression is invalid.
pub fn next_cron_time(cron_expr: &str) -> Option<String> {
    let full_expr = format!("0 {}", cron_expr);
    let schedule = full_expr.parse::<cron::Schedule>().ok()?;
    let tz = TIMEZONE.get().copied().unwrap_or(chrono_tz::UTC);
    let now_local: DateTime<FixedOffset> = Utc::now().with_timezone(&tz).fixed_offset();
    let next = schedule.upcoming(now_local.timezone()).next()?;
    Some(next.format("%Y-%m-%dT%H:%M:%S%:z").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timezone_init_utc() {
        init("UTC");
        let n = now();
        assert!(n.format("%Z").to_string() == "UTC" || n.offset().local_minus_utc() == 0);
    }

    #[test]
    fn test_timezone_init_europe_madrid_parses_correctly() {
        // Test that the parse works, without relying on global OnceLock state
        let tz: Result<chrono_tz::Tz, _> = "Europe/Madrid".parse();
        assert!(tz.is_ok());
    }

    #[test]
    fn test_timezone_now_returns_valid_datetime() {
        init("UTC");
        let n = now();
        // Just verify we get a valid DateTime with no panic
        assert!(n.format("%Y").to_string().len() == 4);
    }

    #[test]
    fn test_timezone_init_empty_falls_back_to_utc() {
        init("");
        let n = now();
        assert_eq!(n.offset().local_minus_utc(), 0);
    }

    #[test]
    fn test_timezone_abbr() {
        init("UTC");
        let abbr = timezone_abbr();
        assert_eq!(abbr, "UTC");
    }

    #[test]
    fn test_now_formatted() {
        init("UTC");
        let s = now_formatted();
        // Should be ISO-like with offset: 2024-01-01T12:00:00+00:00
        assert!(s.contains('T'));
        assert!(s.contains('+') || s.contains('-'));
    }

    #[test]
    fn test_next_cron_time() {
        init("UTC");
        let next = next_cron_time("0 * * * *");
        assert!(next.is_some());
        let s = next.unwrap();
        assert!(s.contains('T'));
        assert!(s.contains('+') || s.contains('-'));
    }

    #[test]
    fn test_next_cron_time_invalid() {
        init("UTC");
        let next = next_cron_time("");
        assert!(next.is_none());
    }

    #[test]
    fn test_now_time_formatted() {
        init("UTC");
        let s = now_time_formatted();
        assert_eq!(s.len(), 8);
    }
}
