use prost::Message;
use snafu::ResultExt;
use yaas::buffed::dto::{
    OauthAuthorizationCodeBuf, OauthAuthorizeBuf, OauthTokenRequestBuf, OauthTokenResponseBuf,
};

use crate::ctx::Ctx;
use crate::error::{HttpClientSnafu, HttpResponseBytesSnafu, ProtobufDecodeSnafu};
use crate::run::AppState;
use crate::{Error, Result};
use yaas::dto::{
    OauthAuthorizationCodeDto, OauthAuthorizeDto, OauthTokenRequestDto, OauthTokenResponseDto,
};

use super::handle_response_error;

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
        return Err(handle_response_error(response, "oauth_codes", Error::InvalidClient).await);
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
        return Err(handle_response_error(response, "oauth_token", Error::InvalidClient).await);
    }

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let token_response =
        OauthTokenResponseBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;
    let dto: OauthTokenResponseDto = token_response.into();

    Ok(dto)
}
