use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Serialize, Deserialize)]
pub struct SuperuserDto {
    pub id: String,
    pub created_at: i64,
}

#[derive(Clone, Serialize, Deserialize, Validate)]
pub struct SetupBodyDto {
    #[validate(length(equal = 36))]
    pub setup_key: String,

    #[validate(length(max = 100))]
    #[validate(email)]
    pub email: String,

    #[validate(length(min = 8, max = 60))]
    pub password: String,
}
