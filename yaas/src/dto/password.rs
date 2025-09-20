use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::buffed::dto::{ChangeCurrentPasswordBuf, PasswordBuf};

#[derive(Clone, Serialize, Deserialize)]
pub struct PasswordDto {
    pub id: i32,
    pub password: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<PasswordBuf> for PasswordDto {
    fn from(pw: PasswordBuf) -> Self {
        PasswordDto {
            id: pw.id,
            password: pw.password,
            created_at: pw.created_at,
            updated_at: pw.updated_at,
        }
    }
}

#[derive(Clone, Deserialize, Validate)]
pub struct NewPasswordDto {
    #[validate(length(min = 8, max = 60))]
    pub password: String,
}

#[derive(Clone, Deserialize, Validate)]
pub struct UpdatePasswordDto {
    #[validate(length(min = 8, max = 60))]
    pub password: Option<String>,
}

#[derive(Clone, Deserialize, Validate)]
pub struct ChangeCurrentPasswordDto {
    #[validate(length(min = 8, max = 60))]
    pub current_password: String,

    #[validate(length(min = 8, max = 60))]
    pub new_password: String,
}

impl From<ChangeCurrentPasswordBuf> for ChangeCurrentPasswordDto {
    fn from(buf: ChangeCurrentPasswordBuf) -> Self {
        ChangeCurrentPasswordDto {
            current_password: buf.current_password,
            new_password: buf.new_password,
        }
    }
}
