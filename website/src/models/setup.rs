use serde::Deserialize;
use validator::Validate;

#[derive(Deserialize, Validate)]
pub struct SetupFormPayload {
    #[validate(email)]
    pub email: String,

    #[validate(length(min = 8))]
    pub password: String,

    pub password_confirm: String,

    #[validate(length(min = 1))]
    pub setup_key: String,
}
