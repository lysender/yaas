use core::fmt;
use serde::{Deserialize, Serialize};
use urlencoding::encode;
use validator::Validate;

use crate::role::Role;
use crate::validators;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrgMemberDto {
    pub id: String,
    pub org_id: String,
    pub user_id: String,
    pub member_email: Option<String>,
    pub member_name: Option<String>,
    pub roles: Vec<Role>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OrgMembershipDto {
    pub org_id: String,
    pub org_name: String,
    pub user_id: String,
    pub roles: Vec<Role>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OrgMemberSuggestionDto {
    pub id: String,
    pub email: String,
    pub name: String,
}

#[derive(Clone, Serialize, Deserialize, Validate)]
pub struct NewOrgMemberDto {
    pub user_id: String,

    #[validate(custom(function = "validators::roles"))]
    pub roles: Vec<String>,

    #[validate(custom(function = "validators::status"))]
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct UpdateOrgMemberDto {
    #[validate(custom(function = "validators::roles"))]
    pub roles: Option<Vec<String>>,

    #[validate(custom(function = "validators::status"))]
    pub status: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Validate)]
pub struct ListOrgMembersParamsDto {
    #[validate(range(min = 1, max = 1000))]
    pub page: Option<i32>,

    #[validate(range(min = 1, max = 50))]
    pub per_page: Option<i32>,

    #[validate(length(min = 0, max = 50))]
    pub keyword: Option<String>,

    pub next: Option<String>,
}

impl Default for ListOrgMembersParamsDto {
    fn default() -> Self {
        Self {
            keyword: None,
            page: Some(1),
            per_page: Some(10),
            next: None,
        }
    }
}

impl fmt::Display for ListOrgMembersParamsDto {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Ideally, we want an empty string if all fields are None
        if self.keyword.is_none()
            && self.page.is_none()
            && self.per_page.is_none()
            && self.next.is_none()
        {
            return write!(f, "");
        }

        let keyword = self.keyword.as_deref().unwrap_or("");
        let page = self.page.unwrap_or(1);
        let per_page = self.per_page.unwrap_or(10);
        let next = self.next.as_deref().unwrap_or("");

        write!(
            f,
            "page={}&per_page={}&keyword={}&next={}",
            page,
            per_page,
            encode(keyword),
            encode(next)
        )
    }
}
