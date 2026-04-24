//! Normalize human input to a Shortcut story external id.
//!
//! Accepts any of:
//!   "SC-123", "sc-123", "123", "  123  "
//!
//! Rejects empty strings, non-digit bodies, and non-positive numbers.

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum IdError {
    #[error("shortcut id cannot be empty")]
    Empty,
    #[error("shortcut id must be positive")]
    NonPositive,
    #[error("shortcut id must be digits (optionally prefixed with SC-): {0}")]
    NotDigits(String),
}

pub fn normalize(raw: &str) -> Result<i64, IdError> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(IdError::Empty);
    }

    let body = match trimmed
        .strip_prefix("SC-")
        .or_else(|| trimmed.strip_prefix("sc-"))
    {
        Some(rest) => rest,
        None => trimmed,
    };

    if body.is_empty() || !body.chars().all(|c| c.is_ascii_digit()) {
        return Err(IdError::NotDigits(trimmed.to_string()));
    }

    let n: i64 = body
        .parse()
        .map_err(|_| IdError::NotDigits(trimmed.to_string()))?;

    if n <= 0 {
        return Err(IdError::NonPositive);
    }

    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_bare_digits() {
        assert_eq!(normalize("123").unwrap(), 123);
    }

    #[test]
    fn strips_sc_prefix_uppercase() {
        assert_eq!(normalize("SC-123").unwrap(), 123);
    }

    #[test]
    fn strips_sc_prefix_lowercase() {
        assert_eq!(normalize("sc-123").unwrap(), 123);
    }

    #[test]
    fn strips_whitespace() {
        assert_eq!(normalize("   SC-123  ").unwrap(), 123);
    }

    #[test]
    fn rejects_empty() {
        assert_eq!(normalize("").unwrap_err(), IdError::Empty);
        assert_eq!(normalize("   ").unwrap_err(), IdError::Empty);
    }

    #[test]
    fn rejects_zero() {
        assert_eq!(normalize("0").unwrap_err(), IdError::NonPositive);
        assert_eq!(normalize("SC-0").unwrap_err(), IdError::NonPositive);
    }

    #[test]
    fn rejects_negative() {
        match normalize("-5") {
            Err(IdError::NotDigits(s)) => assert_eq!(s, "-5"),
            other => panic!("expected NotDigits, got {other:?}"),
        }
    }

    #[test]
    fn rejects_non_digits_body() {
        match normalize("SC-12a") {
            Err(IdError::NotDigits(s)) => assert_eq!(s, "SC-12a"),
            other => panic!("expected NotDigits, got {other:?}"),
        }
    }

    #[test]
    fn rejects_sc_alone() {
        match normalize("SC-") {
            Err(IdError::NotDigits(_)) => {}
            other => panic!("expected NotDigits, got {other:?}"),
        }
    }

    #[test]
    fn rejects_other_prefixes() {
        match normalize("ABC-123") {
            Err(IdError::NotDigits(_)) => {}
            other => panic!("expected NotDigits, got {other:?}"),
        }
    }
}
