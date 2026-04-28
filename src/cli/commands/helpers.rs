use crate::domain::Task;

pub(super) fn status_glyph(t: &Task) -> &'static str {
    if t.completed_at.is_some() {
        "✓"
    } else if t.archived_at.is_some() {
        "·"
    } else {
        " "
    }
}

pub fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}
