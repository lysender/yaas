use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::buffed::dto::{NewUserBuf, UpdateUserBuf, UserBuf};
use crate::validators;

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

#[derive(Clone, Deserialize, Validate)]
pub struct NewUserDto {
    #[validate(email)]
    #[validate(length(min = 1, max = 250))]
    pub email: String,

    #[validate(length(min = 1, max = 100))]
    pub name: String,
}

impl From<NewUserBuf> for NewUserDto {
    fn from(user: NewUserBuf) -> Self {
        NewUserDto {
            email: user.email,
            name: user.name,
        }
    }
}

#[derive(Clone, Deserialize, Validate)]
pub struct UpdateUserDto {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,

    #[validate(custom(function = "validators::status"))]
    pub status: Option<String>,
}

impl From<UpdateUserBuf> for UpdateUserDto {
    fn from(user: UpdateUserBuf) -> Self {
        UpdateUserDto {
            name: user.name,
            status: user.status,
        }
    }
}

#[derive(Clone, Deserialize, Validate)]
pub struct ListUsersParamsDto {
    #[validate(range(min = 1, max = 1000))]
    pub page: Option<i32>,

    #[validate(range(min = 1, max = 50))]
    pub per_page: Option<i32>,

    #[validate(length(min = 0, max = 50))]
    pub keyword: Option<String>,
}
