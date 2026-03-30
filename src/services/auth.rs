use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use snafu::ResultExt;

use crate::dto::{Actor, ActorDto, AuthResponseDto, CredentialsDto, SwitchAuthContextDto};
use crate::{
    Error, Result,
    error::{HttpClientSnafu, HttpResponseParseSnafu},
    run::AppState,
    services::token::decode_auth_token,
};

pub async fn authenticate(state: &AppState, data: CredentialsDto) -> Result<AuthResponseDto> {
    let url = format!("{}/auth/authorize", &state.config.api_url);
    let response = state
        .client
        .post(url.as_str())
        .json(&data)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to process login information. Try again later.".to_string(),
        })?;

    match response.status() {
        StatusCode::OK => {
            response
                .json::<AuthResponseDto>()
                .await
                .context(HttpResponseParseSnafu {
                    msg: "Unable to parse login information.".to_string(),
                })
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
            let actor: ActorDto =
                response
                    .json::<ActorDto>()
                    .await
                    .context(HttpResponseParseSnafu {
                        msg: "Unable to parse auth information.".to_string(),
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
    pub org_id: String,
    pub org_name: String,
    pub next: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SwitchAuthContextParams {
    pub org_id: String,
    pub org_name: String,
    pub next: String,
}

pub async fn switch_auth_context_svc(
    state: &AppState,
    token: &str,
    data: SwitchAuthContextDto,
) -> Result<AuthResponseDto> {
    let url = format!("{}/user/switch-auth-context", &state.config.api_url);

    let response = state
        .client
        .post(url.as_str())
        .json(&data)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to process login information. Try again later.".to_string(),
        })?;

    match response.status() {
        StatusCode::OK => {
            response
                .json::<AuthResponseDto>()
                .await
                .context(HttpResponseParseSnafu {
                    msg: "Unable to parse login information.".to_string(),
                })
        }
        StatusCode::BAD_REQUEST => Err(Error::LoginFailed),
        StatusCode::UNAUTHORIZED => Err(Error::LoginFailed),
        _ => Err("Unable to process login information. Try again later.".into()),
    }
}
