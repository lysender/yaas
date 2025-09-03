use serde::{Deserialize, Serialize};

use crate::buffed::dto::{OrgMemberBuf, OrgMembershipBuf};
use crate::role::Role;
use crate::role::buffed_to_roles;

#[derive(Clone, Serialize, Deserialize)]
pub struct OrgMemberDto {
    pub id: i32,
    pub org_id: i32,
    pub user_id: i32,
    pub roles: Vec<Role>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl TryFrom<OrgMemberBuf> for OrgMemberDto {
    type Error = String;

    fn try_from(member: OrgMemberBuf) -> Result<Self, Self::Error> {
        let Ok(roles) = buffed_to_roles(&member.roles) else {
            return Err("Roles should convert back to enum".to_string());
        };

        Ok(OrgMemberDto {
            id: member.id,
            org_id: member.org_id,
            user_id: member.user_id,
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

    fn try_from(membership: OrgMembershipBuf) -> Result<Self, Self::Error> {
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
