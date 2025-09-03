use axum::{
    Router, middleware,
    routing::{any, get, post},
};

use crate::{
    state::AppState,
    web::{
        handler::{
            authenticate_handler, health_live_handler, health_ready_handler, home_handler,
            not_found_handler, profile_handler, setup_handler, user_authz_handler,
        },
        middleware::{
            app_middleware, auth_middleware, org_app_middleware, org_member_middleware,
            org_middleware, require_auth_middleware, user_middleware,
        },
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

fn users_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(home_handler).post(home_handler))
        .nest("/{user_id}", inner_user_routes(state.clone()))
        .with_state(state)
}

fn inner_user_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(home_handler).patch(home_handler).delete(home_handler),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            user_middleware,
        ))
        .with_state(state)
}

pub fn current_user_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(profile_handler))
        .route("/authz", get(user_authz_handler))
        .route("/change_password", post(home_handler))
        .with_state(state)
}

fn apps_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(home_handler).post(home_handler))
        .nest("/{app_id}", inner_app_routes(state.clone()))
        .with_state(state)
}

fn inner_app_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(home_handler).patch(home_handler).delete(home_handler),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            app_middleware,
        ))
        .with_state(state)
}

fn orgs_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(home_handler).post(home_handler))
        .nest("/{org_id}", inner_org_routes(state.clone()))
        .with_state(state)
}

fn inner_org_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(home_handler).patch(home_handler).delete(home_handler),
        )
        .nest("/members", org_members_routes(state.clone()))
        .nest("/apps", org_apps_routes(state.clone()))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            org_middleware,
        ))
        .with_state(state)
}

fn org_members_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(home_handler).post(home_handler))
        .nest("/{org_member_id}", org_members_inner_routes(state.clone()))
        .with_state(state)
}

fn org_members_inner_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(home_handler).patch(home_handler).delete(home_handler),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            org_member_middleware,
        ))
        .with_state(state)
}

fn org_apps_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(home_handler).post(home_handler))
        .nest("/{org_app_id}", org_apps_inner_routes(state.clone()))
        .with_state(state)
}

fn org_apps_inner_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(home_handler).patch(home_handler).delete(home_handler),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            org_app_middleware,
        ))
        .with_state(state)
}
