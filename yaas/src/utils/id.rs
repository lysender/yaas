use uuid::Uuid;

pub fn generate_id(prefix: &str) -> String {
    format!("{}_{}", prefix, Uuid::now_v7().as_simple())
}

pub fn valid_id(id: &str) -> bool {
    if id.len() != 36 {
        return false;
    }

    // Extract the uuid part starting from the 5th character
    let id = &id[4..];
    let parsed = Uuid::parse_str(id);
    match parsed {
        Ok(val) => match val.get_version() {
            Some(uuid::Version::SortRand) => true,
            _ => false,
        },
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id() {
        // Should be a 36-character prefixed uuid string
        let id = generate_id("usr");
        assert_eq!(id.len(), 36);
        assert!(id.starts_with("usr_"));

        // Can be parsed back as uuid
        assert_eq!(valid_id(id.as_str()), true);
    }
}
