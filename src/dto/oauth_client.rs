use serde::{Deserialize, Serialize};
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, Validate)]
pub struct OauthClientLookupDto {
    #[validate(length(equal = 36))]
    pub client_id: String,

    #[validate(url)]
    #[validate(length(min = 1, max = 250))]
    pub redirect_uri: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OauthClientAppDto {
    pub name: String,
}
