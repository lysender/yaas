use core::fmt;
use serde::{Deserialize, Serialize};
use urlencoding::encode;
use validator::Validate;

use crate::buffed::dto::{NewOrgAppBuf, OrgAppBuf, OrgAppSuggestionBuf};

#[derive(Clone, Serialize, Deserialize)]
pub struct OrgAppDto {
    pub id: i32,
    pub org_id: i32,
    pub app_id: i32,
    pub app_name: Option<String>,
    pub created_at: String,
}

impl From<OrgAppBuf> for OrgAppDto {
    fn from(org_app: OrgAppBuf) -> Self {
        OrgAppDto {
            id: org_app.id,
            org_id: org_app.org_id,
            app_id: org_app.app_id,
            app_name: org_app.app_name,
            created_at: org_app.created_at,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OrgAppSuggestionDto {
    pub id: i32,
    pub name: String,
}

impl From<OrgAppSuggestionBuf> for OrgAppSuggestionDto {
    fn from(suggestion: OrgAppSuggestionBuf) -> Self {
        OrgAppSuggestionDto {
            id: suggestion.id,
            name: suggestion.name,
        }
    }
}

#[derive(Clone, Deserialize, Validate)]
pub struct NewOrgAppDto {
    pub app_id: i32,
}

impl From<NewOrgAppBuf> for NewOrgAppDto {
    fn from(new_org_app: NewOrgAppBuf) -> Self {
        NewOrgAppDto {
            app_id: new_org_app.app_id,
        }
    }
}

#[derive(Clone, Deserialize, Validate)]
pub struct ListOrgAppsParamsDto {
    #[validate(range(min = 1, max = 1000))]
    pub page: Option<i32>,

    #[validate(range(min = 1, max = 50))]
    pub per_page: Option<i32>,

    #[validate(length(min = 0, max = 50))]
    pub keyword: Option<String>,
}

impl Default for ListOrgAppsParamsDto {
    fn default() -> Self {
        Self {
            keyword: None,
            page: Some(1),
            per_page: Some(10),
        }
    }
}

impl fmt::Display for ListOrgAppsParamsDto {
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
