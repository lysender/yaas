use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Clone, Serialize, Deserialize)]
pub struct OauthCodeDto {
    pub id: String,
    pub code: String,
    pub state: String,
    pub redirect_uri: String,
    pub scope: String,
    pub app_id: String,
    pub org_id: String,
    pub user_id: String,
    pub created_at: i64,
    pub expires_at: i64,
}

#[derive(Clone, Deserialize, Validate)]
pub struct NewOauthCodeDto {
    #[validate(length(equal = 36))]
    pub code: String,

    #[validate(length(min = 1, max = 250))]
    pub state: String,

    #[validate(length(min = 1, max = 250))]
    #[validate(url)]
    pub redirect_uri: String,

    #[validate(length(min = 1, max = 250))]
    pub scope: String,

    pub app_id: String,
    pub org_id: String,
    pub user_id: String,
}
