use chrono::{DateTime, Duration, Utc};
use rusqlite::Row;

const CACHE_TTL: Duration = Duration::hours(1);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutStory {
    pub id: i64,
    pub external_id: i64,
    pub title: Option<String>,
    pub epic_name: Option<String>,
    pub state: Option<String>,
    pub fetched_at: DateTime<Utc>,
}

impl ShortcutStory {
    pub fn is_fresh(&self, now: DateTime<Utc>) -> bool {
        now - self.fetched_at < CACHE_TTL
    }
}

impl<'a> TryFrom<&Row<'a>> for ShortcutStory {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'a>) -> Result<Self, Self::Error> {
        Ok(ShortcutStory {
            id: row.get("id")?,
            external_id: row.get("external_id")?,
            title: row.get("title")?,
            epic_name: row.get("epic_name")?,
            state: row.get("state")?,
            fetched_at: row.get("fetched_at")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 4, 22, h, 0, 0).unwrap()
    }

    #[test]
    fn fresh_within_ttl() {
        let s = ShortcutStory {
            id: 1,
            external_id: 123,
            title: None,
            epic_name: None,
            state: None,
            fetched_at: at(10),
        };
        assert!(s.is_fresh(at(10) + Duration::minutes(59)));
    }

    #[test]
    fn stale_past_ttl() {
        let s = ShortcutStory {
            id: 1,
            external_id: 123,
            title: None,
            epic_name: None,
            state: None,
            fetched_at: at(10),
        };
        assert!(!s.is_fresh(at(10) + Duration::minutes(61)));
    }
}
