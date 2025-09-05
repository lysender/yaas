use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::buffed::dto::OrgBuf;

#[derive(Clone, Serialize, Deserialize)]
pub struct OrgDto {
    pub id: i32,
    pub name: String,
    pub status: String,
    pub owner_id: i32,
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
            created_at: org.created_at,
            updated_at: org.updated_at,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct NewOrgDto {
    #[validate(length(min = 1, max = 100))]
    pub name: String,

    pub owner_id: i32,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateOrgDto {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,

    #[validate(length(min = 1, max = 200))]
    pub status: Option<String>,
}
