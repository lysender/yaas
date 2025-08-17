use chrono::{DateTime, Utc};

pub fn datetime_now_str() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

pub fn str_to_datetime(date_str: &str) -> Result<DateTime<Utc>, String> {
    match date_str.parse::<DateTime<Utc>>() {
        Ok(date) => Ok(date),
        Err(_) => Err(format!("Invalid date string: {}", date_str)),
    }
}

pub fn datetime_to_str(date: DateTime<Utc>) -> String {
    date.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_str_to_datetime_valid() {
        let date_str = datetime_now_str();
        let date = str_to_datetime(&date_str);
        assert!(date.is_ok());

        if let Ok(dt) = date {
            assert_eq!(datetime_to_str(dt), date_str);
        }
    }

    #[test]
    fn test_str_to_datetime_invalid() {
        let date_str = "2025-01-01";
        let date = str_to_datetime(&date_str);
        assert!(date.is_err());
    }

    #[test]
    fn test_datetime_to_str() {
        let date_str = datetime_now_str();
        let date = str_to_datetime(&date_str).unwrap();
        let converted_str = datetime_to_str(date);
        assert_eq!(converted_str, date_str);
    }
}
