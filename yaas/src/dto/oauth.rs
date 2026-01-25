use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::buffed::dto::{OauthAuthorizationCodeBuf, OauthAuthorizeBuf};

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct OauthAuthorizeDto {
    #[validate(length(equal = 36))]
    pub client_id: String,

    #[validate(url)]
    #[validate(length(min = 1, max = 250))]
    pub redirect_uri: String,

    #[validate(length(min = 1, max = 250))]
    pub scope: String,

    #[validate(length(min = 1, max = 250))]
    pub state: String,
}

impl From<OauthAuthorizeBuf> for OauthAuthorizeDto {
    fn from(body: OauthAuthorizeBuf) -> Self {
        OauthAuthorizeDto {
            client_id: body.client_id,
            redirect_uri: body.redirect_uri,
            scope: body.scope,
            state: body.state,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct OauthAuthorizationCodeDto {
    pub code: String,
    pub state: String,
}

impl From<OauthAuthorizationCodeBuf> for OauthAuthorizationCodeDto {
    fn from(body: OauthAuthorizationCodeBuf) -> Self {
        OauthAuthorizationCodeDto {
            code: body.code,
            state: body.state,
        }
    }
}
