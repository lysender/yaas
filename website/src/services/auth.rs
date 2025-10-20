use prost::Message;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;

use crate::{
    Error, Result,
    error::{HttpClientSnafu, HttpResponseBytesSnafu, HttpResponseParseSnafu, ProtobufDecodeSnafu},
    run::AppState,
    services::token::decode_auth_token,
};
use yaas::{
    buffed::actor::{ActorBuf, SwitchAuthContextBuf},
    dto::{Actor, AuthResponseDto, CredentialsDto, SwitchAuthContextDto},
};
use yaas::{
    buffed::actor::{AuthResponseBuf, CredentialsBuf},
    dto::ActorDto,
};

pub async fn authenticate(state: &AppState, data: CredentialsDto) -> Result<AuthResponseDto> {
    let body = CredentialsBuf {
        email: data.email,
        password: data.password,
    };

    let url = format!("{}/auth/authorize", &state.config.api_url);
    let response = state
        .client
        .post(url.as_str())
        .body(prost::Message::encode_to_vec(&body))
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to process login information. Try again later.".to_string(),
        })?;

    match response.status() {
        StatusCode::OK => {
            let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
            let buff = AuthResponseBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu)?;
            match buff.try_into() {
                Ok(dto) => Ok(dto),
                Err(e) => Err(Error::Whatever {
                    msg: format!("Unable to parse login information: {}", e),
                }),
            }
        }
        StatusCode::BAD_REQUEST => Err(Error::LoginFailed),
        StatusCode::UNAUTHORIZED => Err(Error::LoginFailed),
        _ => Err("Unable to process login information. Try again later.".into()),
    }
}

pub async fn authenticate_token(state: &AppState, token: &str) -> Result<Actor> {
    let claims = decode_auth_token(token)?;

    // Get from cache first
    if let Some(actor) = state.auth_cache.get(&claims.sub) {
        return Ok(actor);
    }

    let url = format!("{}/user/authz", &state.config.api_url);
    let response = state
        .client
        .get(url.as_str())
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to process auth information. Try again later.".to_string(),
        })?;

    match response.status() {
        StatusCode::OK => {
            let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;

            let buff = ActorBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;
            let actor: ActorDto = buff.try_into().map_err(|e| Error::Whatever {
                msg: format!("Unable to parse auth information: {}", e),
            })?;

            // Store to cache
            state.auth_cache.insert(
                claims.sub,
                Actor {
                    actor: Some(actor.clone()),
                },
            );

            Ok(Actor { actor: Some(actor) })
        }
        StatusCode::UNAUTHORIZED => Err(Error::LoginRequired),
        _ => Err("Unable to process auth information. Try again later.".into()),
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SwitchAuthContextFormData {
    pub token: String,
    pub org_id: i32,
    pub org_name: String,
}

pub async fn switch_auth_context_svc(
    state: &AppState,
    token: &str,
    data: SwitchAuthContextDto,
) -> Result<AuthResponseDto> {
    let url = format!("{}/user/switch-auth-context", &state.config.api_url);
    let body = SwitchAuthContextBuf {
        org_id: data.org_id,
    };

    let response = state
        .client
        .post(url.as_str())
        .body(prost::Message::encode_to_vec(&body))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to process login information. Try again later.".to_string(),
        })?;

    match response.status() {
        StatusCode::OK => {
            let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
            let buff = AuthResponseBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu)?;
            match buff.try_into() {
                Ok(dto) => Ok(dto),
                Err(e) => Err(Error::Whatever {
                    msg: format!("Unable to parse login information: {}", e),
                }),
            }
        }
        StatusCode::BAD_REQUEST => Err(Error::LoginFailed),
        StatusCode::UNAUTHORIZED => Err(Error::LoginFailed),
        _ => Err("Unable to process login information. Try again later.".into()),
    }
}
