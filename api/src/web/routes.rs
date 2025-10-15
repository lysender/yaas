use axum::{
    Router, middleware,
    routing::{any, get, post},
};

use crate::{
    state::AppState,
    web::{
        apps_routes, current_user_routes,
        handler::{
            authenticate_handler, health_live_handler, health_ready_handler, home_handler,
            not_found_handler, setup_handler,
        },
        middleware::{auth_middleware, require_auth_middleware},
        orgs_routes, users_routes,
    },
};

pub fn all_routes(state: AppState) -> Router {
    Router::new()
        .merge(public_routes(state.clone()))
        .merge(private_routes(state.clone()))
        .fallback(any(not_found_handler))
        .with_state(state)
}

fn public_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(home_handler))
        .route("/setup", post(setup_handler))
        .route("/health/liveness", get(health_live_handler))
        .route("/health/readiness", get(health_ready_handler))
        .route("/auth/authorize", post(authenticate_handler))
        .route("/auth/select-org", post(authenticate_handler))
        .with_state(state)
}

fn private_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .nest("/users", users_routes(state.clone()))
        .nest("/user", current_user_routes(state.clone()))
        .nest("/apps", apps_routes(state.clone()))
        .nest("/orgs", orgs_routes(state.clone()))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_auth_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}
