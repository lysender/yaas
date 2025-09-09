use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::buffed::dto::OrgAppBuf;

#[derive(Clone, Serialize, Deserialize)]
pub struct OrgAppDto {
    pub id: i32,
    pub org_id: i32,
    pub app_id: i32,
    pub created_at: String,
}

impl From<OrgAppBuf> for OrgAppDto {
    fn from(org_app: OrgAppBuf) -> Self {
        OrgAppDto {
            id: org_app.id,
            org_id: org_app.org_id,
            app_id: org_app.app_id,
            created_at: org_app.created_at,
        }
    }
}

#[derive(Clone, Deserialize, Validate)]
pub struct NewOrgAppDto {
    pub app_id: i32,
}
