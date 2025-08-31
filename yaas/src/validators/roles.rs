use core::result::Result;
use validator::ValidationError;

use crate::role::to_roles;

pub fn roles(items: &Vec<String>) -> Result<(), ValidationError> {
    match to_roles(items) {
        Ok(_) => Ok(()),
        Err(_) => Err(ValidationError::new("roles")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roles_valid() {
        let items = vec!["OrgAdmin".to_string(), "OrgEditor".to_string()];
        assert!(roles(&items).is_ok());
    }

    #[test]
    fn test_roles_invalid() {
        let items = vec!["OrgAdmin".to_string(), "CEO".to_string()];
        assert!(roles(&items).is_err());
    }
}
