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
    SuperuserKey,
}

impl TryFrom<&str> for IdPrefix {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "omm" => Ok(Self::OrgMember),
            "usr" => Ok(Self::User),
            "app" => Ok(Self::App),
            "cli" => Ok(Self::ClientId),
            "sec" => Ok(Self::ClientSecret),
            "org" => Ok(Self::Org),
            "oap" => Ok(Self::OrgApp),
            "oac" => Ok(Self::OauthCode),
            "pas" => Ok(Self::Password),
            "sup" => Ok(Self::Superuser),
            "suk" => Ok(Self::SuperuserKey),
            _ => Err(format!("Invalid ID Prefix: {value}")),
        }
    }
}

impl core::fmt::Display for IdPrefix {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Self::OrgMember => write!(f, "omm"),
            Self::User => write!(f, "usr"),
            Self::App => write!(f, "app"),
            Self::ClientId => write!(f, "cli"),
            Self::ClientSecret => write!(f, "sec"),
            Self::Org => write!(f, "org"),
            Self::OrgApp => write!(f, "oap"),
            Self::OauthCode => write!(f, "oac"),
            Self::Password => write!(f, "pas"),
            Self::Superuser => write!(f, "sup"),
            Self::SuperuserKey => write!(f, "suk"),
        }
    }
}

pub fn generate_id(prefix: IdPrefix) -> String {
    format!("{}_{}", prefix, Uuid::now_v7().as_simple())
}

pub fn valid_id(id: &str) -> bool {
    if id.len() != 36 {
        return false;
    }

    let Some((prefix, raw_uuid)) = id.split_once('_') else {
        return false;
    };

    if IdPrefix::try_from(prefix).is_err() {
        return false;
    }

    let parsed = Uuid::parse_str(raw_uuid);
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
        assert_eq!(IdPrefix::OrgMember.to_string(), "omm");
        assert_eq!(IdPrefix::User.to_string(), "usr");
        assert_eq!(IdPrefix::App.to_string(), "app");
        assert_eq!(IdPrefix::ClientId.to_string(), "cli");
        assert_eq!(IdPrefix::ClientSecret.to_string(), "sec");
        assert_eq!(IdPrefix::Org.to_string(), "org");
        assert_eq!(IdPrefix::OrgApp.to_string(), "oap");
        assert_eq!(IdPrefix::OauthCode.to_string(), "oac");
        assert_eq!(IdPrefix::Password.to_string(), "pas");
        assert_eq!(IdPrefix::Superuser.to_string(), "sup");
        assert_eq!(IdPrefix::SuperuserKey.to_string(), "suk");
    }

    #[test]
    fn test_invalid_prefix() {
        let id = generate_id(IdPrefix::User);
        let invalid = id.replacen("usr_", "bad_", 1);
        assert!(!valid_id(invalid.as_str()));
    }
}
