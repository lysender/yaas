use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::buffed::dto::{OauthClientAppBuf, OauthClientLookupBuf};

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct OauthClientLookupDto {
    #[validate(length(equal = 36))]
    pub client_id: String,

    #[validate(url)]
    #[validate(length(min = 1, max = 250))]
    pub redirect_uri: String,
}

impl From<OauthClientLookupBuf> for OauthClientLookupDto {
    fn from(body: OauthClientLookupBuf) -> Self {
        OauthClientLookupDto {
            client_id: body.client_id,
            redirect_uri: body.redirect_uri,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct OauthClientAppDto {
    pub name: String,
}

impl From<OauthClientAppBuf> for OauthClientAppDto {
    fn from(body: OauthClientAppBuf) -> Self {
        OauthClientAppDto { name: body.name }
    }
}
