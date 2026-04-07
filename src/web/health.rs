use axum::{Json, Router, extract::State, http::StatusCode, response::IntoResponse, routing::get};

use crate::{
    run::AppState,
    services::health::{HealthChecks, HealthStatus, LiveStatus, check_liveness, check_readiness},
};

pub fn health_api_routes(state: AppState) -> Router {
    Router::new()
        .route("/health/live", get(health_liveness_handler))
        .route("/health/ready", get(health_readiness_handler))
        .with_state(state)
}

pub async fn health_liveness_handler() -> impl IntoResponse {
    match check_liveness().await {
        Ok(status) => (StatusCode::OK, Json(status)),
        Err(_) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(LiveStatus {
                status: "DOWN".to_string(),
            }),
        ),
    }
}

pub async fn health_readiness_handler(State(state): State<AppState>) -> impl IntoResponse {
    match check_readiness(state.db.clone()).await {
        Ok(status) => {
            if status.is_healthy() {
                (StatusCode::OK, Json(status))
            } else {
                (StatusCode::SERVICE_UNAVAILABLE, Json(status))
            }
        }
        Err(err) => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(HealthStatus {
                status: "DOWN".to_string(),
                message: format!("Readiness check failed: {}", err),
                checks: HealthChecks::new(),
            }),
        ),
    }
}
