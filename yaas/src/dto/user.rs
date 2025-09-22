use core::fmt;
use urlencoding::encode;

use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::buffed::dto::{NewUserBuf, NewUserWithPasswordBuf, UpdateUserBuf, UserBuf};
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
pub struct NewUserWithPasswordDto {
    #[validate(email)]
    #[validate(length(min = 1, max = 250))]
    pub email: String,

    #[validate(length(min = 1, max = 100))]
    pub name: String,

    #[validate(length(min = 8, max = 60))]
    pub password: String,
}

impl From<NewUserWithPasswordBuf> for NewUserWithPasswordDto {
    fn from(user: NewUserWithPasswordBuf) -> Self {
        NewUserWithPasswordDto {
            email: user.email,
            name: user.name,
            password: user.password,
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

impl Default for ListUsersParamsDto {
    fn default() -> Self {
        Self {
            keyword: None,
            page: Some(1),
            per_page: Some(10),
        }
    }
}

impl fmt::Display for ListUsersParamsDto {
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
