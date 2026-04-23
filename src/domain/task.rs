use chrono::{DateTime, Utc};
use rusqlite::Row;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: i64,
    pub title: String,
    pub description: Option<String>,
    pub shortcut_story_id: Option<i64>,
    pub completed_at: Option<DateTime<Utc>>,
    pub archived_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Task {
    pub fn is_open(&self) -> bool {
        self.completed_at.is_none() && self.archived_at.is_none()
    }
}

impl<'a> TryFrom<&Row<'a>> for Task {
    type Error = rusqlite::Error;

    fn try_from(row: &Row<'a>) -> Result<Self, Self::Error> {
        Ok(Task {
            id: row.get("id")?,
            title: row.get("title")?,
            description: row.get("description")?,
            shortcut_story_id: row.get("shortcut_story_id")?,
            completed_at: row.get("completed_at")?,
            archived_at: row.get("archived_at")?,
            created_at: row.get("created_at")?,
            updated_at: row.get("updated_at")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn sample() -> Task {
        Task {
            id: 1,
            title: "t".into(),
            description: None,
            shortcut_story_id: None,
            completed_at: None,
            archived_at: None,
            created_at: Utc.with_ymd_and_hms(2026, 4, 22, 10, 0, 0).unwrap(),
            updated_at: Utc.with_ymd_and_hms(2026, 4, 22, 10, 0, 0).unwrap(),
        }
    }

    #[test]
    fn is_open_when_neither_completed_nor_archived() {
        assert!(sample().is_open());
    }

    #[test]
    fn is_not_open_when_completed() {
        let mut t = sample();
        t.completed_at = Some(Utc::now());
        assert!(!t.is_open());
    }

    #[test]
    fn is_not_open_when_archived() {
        let mut t = sample();
        t.archived_at = Some(Utc::now());
        assert!(!t.is_open());
    }
}
