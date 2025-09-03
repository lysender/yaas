use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::buffed::dto::PasswordBuf;

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

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct NewPasswordDto {
    #[validate(length(min = 8, max = 60))]
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdatePasswordDto {
    #[validate(length(min = 8, max = 60))]
    pub password: Option<String>,
}
