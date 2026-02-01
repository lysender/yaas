use prost::Message;
use snafu::ResultExt;
use yaas::buffed::dto::{
    ErrorMessageBuf, OauthAuthorizationCodeBuf, OauthAuthorizeBuf, OauthTokenRequestBuf,
    OauthTokenResponseBuf, UserBuf,
};

use crate::ctx::Ctx;
use crate::error::{HttpClientSnafu, HttpResponseBytesSnafu, ProtobufDecodeSnafu};
use crate::run::AppState;
use crate::{Error, Result};
use yaas::dto::{
    OauthAuthorizationCodeDto, OauthAuthorizeDto, OauthTokenRequestDto, OauthTokenResponseDto,
    UserDto,
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

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let auth_code =
        OauthAuthorizationCodeBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;
    let dto: OauthAuthorizationCodeDto = auth_code.into();

    Ok(dto)
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

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let token_response =
        OauthTokenResponseBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;
    let dto: OauthTokenResponseDto = token_response.into();

    Ok(dto)
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

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let user = UserBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;
    let dto: UserDto = user.into();

    Ok(dto)
}

async fn handle_oauth_error(response: reqwest::Response) -> Error {
    let Some(content_type) = response.headers().get("Content-Type") else {
        return Error::Service {
            msg: "Unable to identify service response type".to_string(),
        };
    };

    let Ok(content_type) = content_type.to_str() else {
        return Error::Service {
            msg: "Unable to identify service response type".to_string(),
        };
    };

    // We only expect a protobuf response or a text response
    match content_type {
        "application/x-protobuf" => {
            let Ok(body_bytes) = response.bytes().await else {
                return Error::Service {
                    msg: "Unable to read protobuf service error response".to_string(),
                };
            };
            let Ok(msg) = ErrorMessageBuf::decode(&body_bytes[..]) else {
                return Error::Service {
                    msg: "Unable to decode protobuf service error response".to_string(),
                };
            };

            Error::Oauth { msg: msg.message }
        }
        "text/plain" | "text/plain; charset=utf-8" => {
            // Probably some default http error
            let text_res = response.text().await;
            Error::Service {
                msg: match text_res {
                    Ok(text) => text,
                    Err(_) => "Unable to parse text service error response".to_string(),
                },
            }
        }
        _ => Error::Service {
            msg: "Unable to parse service error response".to_string(),
        },
    }
}
