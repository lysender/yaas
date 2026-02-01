use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::buffed::dto::{
    OauthAuthorizationCodeBuf, OauthAuthorizeBuf, OauthTokenRequestBuf, OauthTokenResponseBuf,
};

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

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct OauthTokenRequestDto {
    #[validate(length(equal = 36))]
    pub client_id: String,

    #[validate(length(equal = 36))]
    pub client_secret: String,

    #[validate(length(equal = 36))]
    pub code: String,

    #[validate(length(min = 1, max = 250))]
    pub state: String,

    #[validate(url)]
    #[validate(length(min = 1, max = 250))]
    pub redirect_uri: String,
}

impl From<OauthTokenRequestBuf> for OauthTokenRequestDto {
    fn from(body: OauthTokenRequestBuf) -> Self {
        OauthTokenRequestDto {
            client_id: body.client_id,
            client_secret: body.client_secret,
            code: body.code,
            state: body.state,
            redirect_uri: body.redirect_uri,
        }
    }
}

#[derive(Clone, Serialize)]
pub struct OauthTokenResponseDto {
    pub access_token: String,
    pub scope: String,
    pub token_type: String,
}

impl From<OauthTokenResponseBuf> for OauthTokenResponseDto {
    fn from(body: OauthTokenResponseBuf) -> Self {
        OauthTokenResponseDto {
            access_token: body.access_token,
            scope: body.scope,
            token_type: body.token_type,
        }
    }
}
