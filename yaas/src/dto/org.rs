use serde::{Deserialize, Serialize};

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
