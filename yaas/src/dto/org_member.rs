use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::buffed::dto::{NewOrgMemberBuf, OrgMemberBuf, OrgMembershipBuf, UpdateOrgMemberBuf};
use crate::role::Role;
use crate::role::buffed_to_roles;
use crate::validators;

#[derive(Clone, Serialize, Deserialize)]
pub struct OrgMemberDto {
    pub id: i32,
    pub org_id: i32,
    pub user_id: i32,
    pub name: Option<String>,
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

#[derive(Clone, Deserialize, Validate)]
pub struct NewOrgMemberDto {
    pub user_id: i32,

    #[validate(length(min = 1, max = 20))]
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
    #[validate(length(min = 1, max = 20))]
    #[validate(custom(function = "validators::roles"))]
    pub roles: Option<Vec<String>>,

    #[validate(custom(function = "validators::status"))]
    pub status: Option<String>,
}

impl TryFrom<UpdateOrgMemberBuf> for UpdateOrgMemberDto {
    type Error = String;

    fn try_from(member: UpdateOrgMemberBuf) -> std::result::Result<Self, Self::Error> {
        let Ok(roles) = buffed_to_roles(&member.roles) else {
            return Err("Roles should convert back to enum".to_string());
        };

        Ok(UpdateOrgMemberDto {
            roles: Some(roles.iter().map(|r| r.to_string()).collect()),
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
