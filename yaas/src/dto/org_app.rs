use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::buffed::dto::{NewOrgAppBuf, OrgAppBuf};

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
