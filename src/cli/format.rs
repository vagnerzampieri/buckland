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

/// Render a Unicode-block bar of `width` cells representing `value` against
/// `max`. Empty when `value` or `max` is zero. Uses partial blocks for
/// fractional widths so a 0.3-cell value still renders.
pub fn bar(value: i64, max: i64, width: usize) -> String {
    if value <= 0 || max <= 0 || width == 0 {
        return String::new();
    }
    let proportion = (value as f64 / max as f64).min(1.0);
    let cells = proportion * width as f64;
    let full = cells.floor() as usize;
    let remainder = cells - full as f64;

    // 8 partial-block glyphs from 1/8 to 7/8 of a cell.
    const PARTIALS: [char; 8] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];
    let partial_idx = (remainder * 8.0).round() as usize;

    let mut out = String::with_capacity(width * 4);
    for _ in 0..full {
        out.push('█');
    }
    if full < width && partial_idx > 0 {
        out.push(PARTIALS[partial_idx.min(7)]);
    }
    out
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

    #[test]
    fn bar_full_width_when_value_equals_max() {
        // 30 full blocks expected.
        let s = bar(100, 100, 30);
        assert_eq!(s.chars().count(), 30);
        assert!(s.chars().all(|c| c == '█'));
    }

    #[test]
    fn bar_empty_when_value_zero() {
        let s = bar(0, 100, 30);
        assert_eq!(s, "");
    }

    #[test]
    fn bar_empty_when_max_zero() {
        let s = bar(0, 0, 30);
        assert_eq!(s, "");
    }

    #[test]
    fn bar_half_width_when_value_is_half_of_max() {
        let s = bar(50, 100, 30);
        let chars: Vec<char> = s.chars().collect();
        // 15 full blocks
        assert_eq!(chars.iter().take_while(|c| **c == '█').count(), 15);
    }

    #[test]
    fn bar_uses_partial_block_when_fractional() {
        // 10/100 of 30 = 3.0 → exactly 3 full blocks, no trailing partial.
        let s = bar(10, 100, 30);
        assert_eq!(s.chars().count(), 3);
        // 1/100 of 30 = 0.3 → 0 full blocks + one partial.
        let s = bar(1, 100, 30);
        assert_eq!(s.chars().count(), 1);
        let c = s.chars().next().unwrap();
        assert!("▏▎▍▌▋▊▉".contains(c), "got {c:?}");
    }
}
