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
use yaas::{actor::Actor, buffed::actor::ActorBuf};
use yaas::{
    actor::ActorDto,
    buffed::actor::{AuthResponseBuf, CredentialsBuf},
};

#[derive(Serialize)]
pub struct AuthPayload {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct AuthResponse {
    pub token: String,
}

pub async fn authenticate(state: &AppState, data: AuthPayload) -> Result<AuthResponse> {
    let body = CredentialsBuf {
        email: data.username,
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
            let auth = AuthResponse {
                token: buff.token.expect("token is expected after authentication"),
            };

            Ok(auth)
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
