use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdPrefix {
    OrgMember,
    User,
    App,
    ClientId,
    ClientSecret,
    Org,
    OrgApp,
    OauthCode,
    Password,
    Superuser,
}

impl IdPrefix {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OrgMember => "omm",
            Self::User => "usr",
            Self::App => "app",
            Self::ClientId => "cli",
            Self::ClientSecret => "sec",
            Self::Org => "org",
            Self::OrgApp => "oap",
            Self::OauthCode => "oac",
            Self::Password => "pas",
            Self::Superuser => "sup",
        }
    }
}

pub fn generate_id(prefix: IdPrefix) -> String {
    format!("{}_{}", prefix.as_str(), Uuid::now_v7().as_simple())
}

pub fn valid_id(id: &str) -> bool {
    if id.len() != 36 {
        return false;
    }

    // Extract the uuid part starting from the 5th character
    let id = &id[4..];
    let parsed = Uuid::parse_str(id);
    match parsed {
        Ok(val) => matches!(val.get_version(), Some(uuid::Version::SortRand)),
        Err(_) => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_id() {
        // Should be a 36-character prefixed uuid string
        let id = generate_id(IdPrefix::User);
        assert_eq!(id.len(), 36);
        assert!(id.starts_with("usr_"));

        // Can be parsed back as uuid
        assert_eq!(valid_id(id.as_str()), true);
    }

    #[test]
    fn test_id_prefix_mapping() {
        assert_eq!(IdPrefix::OrgMember.as_str(), "omm");
        assert_eq!(IdPrefix::User.as_str(), "usr");
        assert_eq!(IdPrefix::App.as_str(), "app");
        assert_eq!(IdPrefix::ClientId.as_str(), "cli");
        assert_eq!(IdPrefix::ClientSecret.as_str(), "sec");
        assert_eq!(IdPrefix::Org.as_str(), "org");
        assert_eq!(IdPrefix::OrgApp.as_str(), "oap");
        assert_eq!(IdPrefix::OauthCode.as_str(), "oac");
        assert_eq!(IdPrefix::Password.as_str(), "pas");
        assert_eq!(IdPrefix::Superuser.as_str(), "sup");
    }
}
