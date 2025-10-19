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
    buffed::{
        actor::{AuthResponseBuf, CredentialsBuf},
        dto::{ErrorMessageBuf, SetupBodyBuf, SuperuserBuf, UserBuf},
    },
    dto::{CredentialsDto, SetupBodyDto},
    validators::flatten_errors,
};

use crate::{
    Error, Result,
    auth::authenticate,
    error::ValidationSnafu,
    health::{check_liveness, check_readiness},
    services::superuser::setup_superuser_svc,
    state::AppState,
    web::{build_response, response::JsonResponse},
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
    let error_message = ErrorMessageBuf {
        status_code: StatusCode::NOT_FOUND.as_u16() as u32,
        message: "Not Found".to_string(),
        error: "Not Found".to_string(),
        error_code: None,
    };

    Ok(Response::builder()
        .status(404)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(error_message.encode_to_vec()))
        .unwrap())
}

pub async fn health_live_handler() -> Result<JsonResponse> {
    let health = check_liveness().await?;
    Ok(JsonResponse::new(serde_json::to_string(&health).unwrap()))
}

pub async fn health_ready_handler(State(state): State<AppState>) -> Result<JsonResponse> {
    let health = check_readiness(state.db).await?;
    let status = if health.is_healthy() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    Ok(JsonResponse::with_status(
        status,
        serde_json::to_string(&health).unwrap(),
    ))
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
    let buffed_auth_res = AuthResponseBuf {
        user: Some(UserBuf {
            id: auth_res.user.id,
            email: auth_res.user.email,
            name: auth_res.user.name,
            status: auth_res.user.status,
            created_at: auth_res.user.created_at,
            updated_at: auth_res.user.updated_at,
        }),
        token: auth_res.token,
        org_id: auth_res.org_id,
        org_count: auth_res.org_count,
    };

    Ok(build_response(200, buffed_auth_res.encode_to_vec()))
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
    let buffed_superuser = SuperuserBuf {
        id: superuser.id,
        created_at: superuser.created_at,
    };

    Ok(build_response(200, buffed_superuser.encode_to_vec()))
}
