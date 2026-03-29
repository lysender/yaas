use snafu::ResultExt;
use yaas::buffed::dto::{OauthAuthorizeBuf, OauthClientLookupBuf, OauthTokenRequestBuf};

use crate::ctx::Ctx;
use crate::error::{HttpClientSnafu, HttpResponseParseSnafu};
use crate::run::AppState;
use crate::{Error, Result};
use yaas::dto::{
    ErrorMessageDto, OauthAuthorizationCodeDto, OauthAuthorizeDto, OauthClientAppDto,
    OauthClientLookupDto, OauthTokenRequestDto, OauthTokenResponseDto, UserDto,
};

pub async fn create_authorization_code(
    state: &AppState,
    ctx: &Ctx,
    query: &OauthAuthorizeDto,
) -> Result<OauthAuthorizationCodeDto> {
    let token = ctx.token().expect("Token is required");
    let url = format!("{}/oauth/authorize", &state.config.api_url);

    let body = OauthAuthorizeBuf {
        client_id: query.client_id.clone(),
        redirect_uri: query.redirect_uri.clone(),
        scope: query.scope.clone(),
        state: query.state.clone(),
    };

    let response = state
        .client
        .post(url)
        .bearer_auth(token)
        .body(prost::Message::encode_to_vec(&body))
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to create authorization code. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_oauth_error(response).await);
    }

    response
        .json::<OauthAuthorizationCodeDto>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse oauth authorization code response.".to_string(),
        })
}

pub async fn exchange_code_for_access_token(
    state: &AppState,
    payload: &OauthTokenRequestDto,
) -> Result<OauthTokenResponseDto> {
    let url = format!("{}/oauth/token", &state.config.api_url);

    let body = OauthTokenRequestBuf {
        client_id: payload.client_id.clone(),
        client_secret: payload.client_secret.clone(),
        code: payload.code.clone(),
        redirect_uri: payload.redirect_uri.clone(),
        state: payload.state.clone(),
    };

    let response = state
        .client
        .post(url)
        .body(prost::Message::encode_to_vec(&body))
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to exchange token. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_oauth_error(response).await);
    }

    response
        .json::<OauthTokenResponseDto>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse oauth token response.".to_string(),
        })
}

pub async fn oauth_profile(state: &AppState, token: &str) -> Result<UserDto> {
    let url = format!("{}/user", &state.config.api_url);

    let response = state
        .client
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to fetch oauth profile. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_oauth_error(response).await);
    }

    response
        .json::<UserDto>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse oauth profile response.".to_string(),
        })
}

pub async fn lookup_oauth_client_app(
    state: &AppState,
    payload: &OauthClientLookupDto,
) -> Result<OauthClientAppDto> {
    let url = format!("{}/oauth/client", &state.config.api_url);

    let body = OauthClientLookupBuf {
        client_id: payload.client_id.clone(),
        redirect_uri: payload.redirect_uri.clone(),
    };

    let response = state
        .client
        .post(url)
        .body(prost::Message::encode_to_vec(&body))
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to validate oauth client. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_oauth_error(response).await);
    }

    response
        .json::<OauthClientAppDto>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse oauth client response.".to_string(),
        })
}

async fn handle_oauth_error(response: reqwest::Response) -> Error {
    let content_type = response
        .headers()
        .get("Content-Type")
        .and_then(|header| header.to_str().ok())
        .unwrap_or("");

    if content_type.starts_with("application/json") {
        let json = response.json::<ErrorMessageDto>().await;
        return match json {
            Ok(err) => Error::Oauth { msg: err.message },
            Err(_) => Error::Service {
                msg: "Unable to parse oauth error response".to_string(),
            },
        };
    }

    let text_res = response.text().await;
    match text_res {
        Ok(text) => {
            if let Ok(json) = serde_json::from_str::<ErrorMessageDto>(&text) {
                return Error::Oauth { msg: json.message };
            }

            if !text.is_empty() {
                return Error::Service { msg: text };
            }

            Error::Service {
                msg: "Unable to parse service error response".to_string(),
            }
        }
        Err(_) => Error::Service {
            msg: "Unable to parse service error response".to_string(),
        },
    }
}
