use axum::{
    body::{Body, Bytes},
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use prost::Message;
use serde::Serialize;
use snafu::ensure;
use validator::Validate;

use yaas::{
    buffed::{actor::CredentialsBuf, dto::SetupBodyBuf},
    dto::{CredentialsDto, ErrorMessageDto, SetupBodyDto, SetupStatusDto, SuperuserDto},
    validators::flatten_errors,
};

use crate::{
    Error, Result,
    auth::authenticate,
    error::ValidationSnafu,
    health::{check_liveness, check_readiness},
    services::superuser::{setup_status_svc, setup_superuser_svc},
    state::AppState,
    web::json_response,
};

#[derive(Serialize)]
pub struct AppMeta {
    pub name: String,
    pub version: String,
}

pub async fn home_handler() -> impl IntoResponse {
    Json(AppMeta {
        name: "yaas".to_string(),
        version: "0.1.0".to_string(),
    })
}

pub async fn not_found_handler(State(_state): State<AppState>) -> Result<Response<Body>> {
    let error_message = ErrorMessageDto {
        status_code: StatusCode::NOT_FOUND.as_u16(),
        message: "Not Found".to_string(),
        error: "Not Found".to_string(),
        error_code: None,
    };

    Ok(json_response(StatusCode::NOT_FOUND, error_message))
}

pub async fn health_live_handler() -> Result<Response<Body>> {
    let health = check_liveness().await?;
    Ok(json_response(StatusCode::OK, health))
}

pub async fn health_ready_handler(State(state): State<AppState>) -> Result<Response<Body>> {
    let health = check_readiness(state.db).await?;
    let status = if health.is_healthy() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    Ok(json_response(status, health))
}

pub async fn authenticate_handler(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<Response<Body>> {
    // Parse body as protobuf message
    let Ok(creds) = CredentialsBuf::decode(body) else {
        return Err(Error::BadProtobuf);
    };

    let credentials = CredentialsDto {
        email: creds.email,
        password: creds.password,
    };

    let errors = credentials.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let auth_res = authenticate(&state, &credentials).await?;
    Ok(json_response(StatusCode::OK, auth_res))
}

pub async fn setup_handler(State(state): State<AppState>, body: Bytes) -> Result<Response<Body>> {
    // Parse body as protobuf message
    let Ok(payload) = SetupBodyBuf::decode(body) else {
        return Err(Error::BadProtobuf);
    };

    let payload = SetupBodyDto {
        setup_key: payload.setup_key,
        email: payload.email,
        password: payload.password,
    };

    let errors = payload.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let superuser = setup_superuser_svc(&state, payload).await?;
    Ok(json_response(
        StatusCode::OK,
        SuperuserDto {
            id: superuser.id,
            created_at: superuser.created_at,
        },
    ))
}

pub async fn setup_status_handler(State(state): State<AppState>) -> Result<Response<Body>> {
    let done = setup_status_svc(&state).await?;
    Ok(json_response(StatusCode::OK, SetupStatusDto { done }))
}
