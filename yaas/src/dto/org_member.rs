use core::fmt;
use serde::{Deserialize, Serialize};
use urlencoding::encode;
use validator::Validate;

use crate::buffed::dto::{
    NewOrgMemberBuf, OrgMemberBuf, OrgMemberSuggestionBuf, OrgMembershipBuf, UpdateOrgMemberBuf,
};
use crate::role::Role;
use crate::role::buffed_to_roles;
use crate::validators;

#[derive(Clone, Serialize, Deserialize)]
pub struct OrgMemberDto {
    pub id: i32,
    pub org_id: i32,
    pub user_id: i32,
    pub name: Option<String>,
    pub email: Option<String>,
    pub roles: Vec<Role>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl TryFrom<OrgMemberBuf> for OrgMemberDto {
    type Error = String;

    fn try_from(member: OrgMemberBuf) -> std::result::Result<Self, Self::Error> {
        let Ok(roles) = buffed_to_roles(&member.roles) else {
            return Err("Roles should convert back to enum".to_string());
        };

        Ok(OrgMemberDto {
            id: member.id,
            org_id: member.org_id,
            user_id: member.user_id,
            name: member.name,
            email: member.email,
            roles,
            status: member.status,
            created_at: member.created_at,
            updated_at: member.updated_at,
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OrgMembershipDto {
    pub org_id: i32,
    pub org_name: String,
    pub user_id: i32,
    pub roles: Vec<Role>,
}

impl TryFrom<OrgMembershipBuf> for OrgMembershipDto {
    type Error = String;

    fn try_from(membership: OrgMembershipBuf) -> std::result::Result<Self, Self::Error> {
        let Ok(roles) = buffed_to_roles(&membership.roles) else {
            return Err("Roles should convert back to enum".to_string());
        };

        Ok(OrgMembershipDto {
            org_id: membership.org_id,
            org_name: membership.org_name,
            user_id: membership.user_id,
            roles,
        })
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OrgMemberSuggestionDto {
    pub id: i32,
    pub name: String,
    pub email: String,
}

impl From<OrgMemberSuggestionBuf> for OrgMemberSuggestionDto {
    fn from(suggestion: OrgMemberSuggestionBuf) -> Self {
        OrgMemberSuggestionDto {
            id: suggestion.id,
            name: suggestion.name,
            email: suggestion.email,
        }
    }
}

#[derive(Clone, Deserialize, Validate)]
pub struct NewOrgMemberDto {
    pub user_id: i32,

    #[validate(custom(function = "validators::roles"))]
    pub roles: Vec<String>,

    #[validate(custom(function = "validators::status"))]
    pub status: String,
}

impl TryFrom<NewOrgMemberBuf> for NewOrgMemberDto {
    type Error = String;

    fn try_from(member: NewOrgMemberBuf) -> std::result::Result<Self, Self::Error> {
        let Ok(roles) = buffed_to_roles(&member.roles) else {
            return Err("Roles should convert back to enum".to_string());
        };

        Ok(NewOrgMemberDto {
            user_id: member.user_id,
            roles: roles.iter().map(|r| r.to_string()).collect(),
            status: member.status,
        })
    }
}

#[derive(Clone, Deserialize, Validate)]
pub struct UpdateOrgMemberDto {
    #[validate(custom(function = "validators::roles"))]
    pub roles: Option<Vec<String>>,

    #[validate(custom(function = "validators::status"))]
    pub status: Option<String>,
}

impl TryFrom<UpdateOrgMemberBuf> for UpdateOrgMemberDto {
    type Error = String;

    fn try_from(member: UpdateOrgMemberBuf) -> std::result::Result<Self, Self::Error> {
        let mut roles: Option<Vec<String>> = None;

        // Empty roles means no change as roles are required
        let Ok(parsed_roles) = buffed_to_roles(&member.roles) else {
            return Err("Roles should convert back to enum".to_string());
        };
        if parsed_roles.len() > 0 {
            roles = Some(parsed_roles.iter().map(|r| r.to_string()).collect());
        }

        Ok(UpdateOrgMemberDto {
            roles,
            status: member.status,
        })
    }
}

#[derive(Clone, Deserialize, Validate)]
pub struct ListOrgMembersParamsDto {
    #[validate(range(min = 1, max = 1000))]
    pub page: Option<i32>,

    #[validate(range(min = 1, max = 50))]
    pub per_page: Option<i32>,

    #[validate(length(min = 0, max = 50))]
    pub keyword: Option<String>,
}

impl Default for ListOrgMembersParamsDto {
    fn default() -> Self {
        Self {
            keyword: None,
            page: Some(1),
            per_page: Some(10),
        }
    }
}

impl fmt::Display for ListOrgMembersParamsDto {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Ideally, we want an empty string if all fields are None
        if self.keyword.is_none() && self.page.is_none() && self.per_page.is_none() {
            return write!(f, "");
        }

        let keyword = self.keyword.as_deref().unwrap_or("");
        let page = self.page.unwrap_or(1);
        let per_page = self.per_page.unwrap_or(10);

        write!(
            f,
            "page={}&per_page={}&keyword={}",
            page,
            per_page,
            encode(keyword)
        )
    }
}
