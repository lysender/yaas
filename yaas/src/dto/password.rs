use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Serialize, Deserialize)]
pub struct PasswordDto {
    pub id: String,
    pub password: String,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Clone, Serialize, Deserialize, Validate)]
pub struct NewPasswordDto {
    #[validate(length(min = 8, max = 60))]
    pub password: String,
}

#[derive(Clone, Serialize, Deserialize, Validate)]
pub struct ChangeCurrentPasswordDto {
    #[validate(length(min = 8, max = 60))]
    pub current_password: String,

    #[validate(length(min = 8, max = 60))]
    pub new_password: String,
}
