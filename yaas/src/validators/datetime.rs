use chrono::{DateTime, Utc};
use core::result::Result;
use validator::ValidationError;

pub fn datetime(value: &str) -> Result<(), ValidationError> {
    if value.len() == 0 {
        return Err(ValidationError::new("datetime"));
    }

    match value.parse::<DateTime<Utc>>() {
        Ok(_) => Ok(()),
        Err(_) => Err(ValidationError::new("datetime")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_datetime() {
        let today = Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        assert!(datetime(&today).is_ok());
        assert!(datetime("2025-01-01").is_err());
        assert!(datetime("").is_err());
    }
}
