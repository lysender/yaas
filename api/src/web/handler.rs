use axum::{
    body::Body,
    extract::{Json, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use yaas::dto::{CredentialsDto, ErrorMessageDto, SetupBodyDto, SetupStatusDto, SuperuserDto};

use crate::{
    Result,
    auth::authenticate,
    health::{check_liveness, check_readiness},
    services::superuser::{setup_status_svc, setup_superuser_svc},
    state::AppState,
    web::{json_input::validate_json_payload, json_response},
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
    payload: crate::web::json_input::JsonPayload<CredentialsDto>,
) -> Result<Response<Body>> {
    let credentials = validate_json_payload(payload)?;

    let auth_res = authenticate(&state, &credentials).await?;
    Ok(json_response(StatusCode::OK, auth_res))
}

pub async fn setup_handler(
    State(state): State<AppState>,
    payload: crate::web::json_input::JsonPayload<SetupBodyDto>,
) -> Result<Response<Body>> {
    let payload = validate_json_payload(payload)?;

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
