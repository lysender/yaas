use core::fmt;
use serde::{Deserialize, Serialize};
use urlencoding::encode;
use validator::Validate;

use crate::buffed::dto::{NewOrgBuf, OrgBuf, OrgOwnerSuggestionBuf, UpdateOrgBuf};

#[derive(Clone, Serialize, Deserialize)]
pub struct OrgDto {
    pub id: i32,
    pub name: String,
    pub status: String,
    pub owner_id: Option<i32>,
    pub owner_email: Option<String>,
    pub owner_name: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

impl From<OrgBuf> for OrgDto {
    fn from(org: OrgBuf) -> Self {
        OrgDto {
            id: org.id,
            name: org.name,
            status: org.status,
            owner_id: org.owner_id,
            owner_email: org.owner_email,
            owner_name: org.owner_name,
            created_at: org.created_at,
            updated_at: org.updated_at,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OrgOwnerSuggestionDto {
    pub id: i32,
    pub email: String,
    pub name: String,
}

impl From<OrgOwnerSuggestionBuf> for OrgOwnerSuggestionDto {
    fn from(suggestion: OrgOwnerSuggestionBuf) -> Self {
        OrgOwnerSuggestionDto {
            id: suggestion.id,
            email: suggestion.email,
            name: suggestion.name,
        }
    }
}

#[derive(Clone, Deserialize, Validate)]
pub struct NewOrgDto {
    #[validate(length(min = 1, max = 100))]
    pub name: String,

    pub owner_id: i32,
}

impl From<NewOrgBuf> for NewOrgDto {
    fn from(org: NewOrgBuf) -> Self {
        NewOrgDto {
            name: org.name,
            owner_id: org.owner_id,
        }
    }
}

#[derive(Clone, Deserialize, Validate)]
pub struct UpdateOrgDto {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,

    #[validate(length(min = 1, max = 200))]
    pub status: Option<String>,

    pub owner_id: Option<i32>,
}

impl From<UpdateOrgBuf> for UpdateOrgDto {
    fn from(org: UpdateOrgBuf) -> Self {
        UpdateOrgDto {
            name: org.name,
            status: org.status,
            owner_id: org.owner_id,
        }
    }
}

#[derive(Clone, Deserialize, Validate)]
pub struct ListOrgsParamsDto {
    #[validate(range(min = 1, max = 1000))]
    pub page: Option<i32>,

    #[validate(range(min = 1, max = 50))]
    pub per_page: Option<i32>,

    #[validate(length(min = 0, max = 50))]
    pub keyword: Option<String>,
}

impl Default for ListOrgsParamsDto {
    fn default() -> Self {
        Self {
            keyword: None,
            page: Some(1),
            per_page: Some(10),
        }
    }
}

impl fmt::Display for ListOrgsParamsDto {
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

#[derive(Clone, Deserialize, Validate)]
pub struct ListOrgOwnerSuggestionsParamsDto {
    #[validate(range(min = 1, max = 1000))]
    pub page: Option<i32>,

    #[validate(range(min = 1, max = 50))]
    pub per_page: Option<i32>,

    #[validate(length(min = 0, max = 50))]
    pub keyword: Option<String>,

    pub exclude_id: Option<i32>,
}

impl Default for ListOrgOwnerSuggestionsParamsDto {
    fn default() -> Self {
        Self {
            keyword: None,
            page: Some(1),
            per_page: Some(10),
            exclude_id: None,
        }
    }
}

impl fmt::Display for ListOrgOwnerSuggestionsParamsDto {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Ideally, we want an empty string if all fields are None
        if self.keyword.is_none() && self.page.is_none() && self.per_page.is_none() {
            return write!(f, "");
        }

        let keyword = self.keyword.as_deref().unwrap_or("");
        let page = self.page.unwrap_or(1);
        let per_page = self.per_page.unwrap_or(10);
        let exclude_id_str = match self.exclude_id {
            Some(id) => format!("&exclude_id={}", id),
            None => "".to_string(),
        };

        write!(
            f,
            "page={}&per_page={}&keyword={}{}",
            page,
            per_page,
            encode(keyword),
            exclude_id_str
        )
    }
}
