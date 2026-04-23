use chrono::Duration;

/// Format as "1h 23m" or "12m" or "5s". Used in list rows and summaries.
pub fn duration_compact(d: Duration) -> String {
    let secs = d.num_seconds().max(0);
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    if h > 0 {
        format!("{h}h {m:02}m")
    } else if m > 0 {
        format!("{m}m")
    } else {
        format!("{s}s")
    }
}

/// Format as "HH:MM:SS". Used for the active-timer header/status line.
pub fn duration_hms(d: Duration) -> String {
    let secs = d.num_seconds().max(0);
    let h = secs / 3600;
    let m = (secs % 3600) / 60;
    let s = secs % 60;
    format!("{h:02}:{m:02}:{s:02}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_picks_the_right_unit() {
        assert_eq!(duration_compact(Duration::seconds(0)), "0s");
        assert_eq!(duration_compact(Duration::seconds(45)), "45s");
        assert_eq!(duration_compact(Duration::minutes(12)), "12m");
        assert_eq!(duration_compact(Duration::minutes(83)), "1h 23m");
    }

    #[test]
    fn compact_clamps_negative() {
        assert_eq!(duration_compact(Duration::seconds(-30)), "0s");
    }

    #[test]
    fn hms_pads() {
        assert_eq!(duration_hms(Duration::seconds(5)), "00:00:05");
        assert_eq!(duration_hms(Duration::seconds(3725)), "01:02:05");
    }
}
