use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::buffed::dto::{
    AppBuf, ErrorMessageBuf, OauthCodeBuf, OrgAppBuf, OrgBuf, OrgMemberBuf, OrgMembershipBuf,
    PasswordBuf, SetupBodyBuf, SuperuserBuf, UserBuf,
};
use crate::role::Role;
use crate::role::buffed_to_roles;

#[derive(Clone, Serialize, Deserialize)]
pub struct UserDto {
    pub id: i32,
    pub email: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<UserBuf> for UserDto {
    fn from(user: UserBuf) -> Self {
        UserDto {
            id: user.id,
            email: user.email,
            name: user.name,
            status: user.status,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}
