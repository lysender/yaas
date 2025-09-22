use axum::extract::State;
use axum::handler::HandlerWithoutStateExt;
use axum::http::HeaderMap;
use axum::response::{IntoResponse, Response};
use axum::routing::{any, get, get_service, post};
use axum::{Extension, Router, middleware};
use reqwest::StatusCode;
use std::path::PathBuf;
use tower_http::services::{ServeDir, ServeFile};
use tracing::error;

use crate::ctx::Ctx;
use crate::error::ErrorInfo;
use crate::models::Pref;
use crate::run::AppState;
use crate::web::middleware::app_middleware;
use crate::web::profile::{change_current_password_handler, post_change_current_password_handler};
use crate::web::users::{post_change_password_handler, search_users_handler};
use crate::web::{
    app_controls_handler, app_page_handler, apps_handler, delete_app_handler, error_handler,
    index_handler, login_handler, logout_handler, new_app_handler, post_delete_app_handler,
    post_login_handler, post_new_app_handler, post_regenerate_app_secret_handler,
    post_update_app_handler, regenerate_app_secret_handler, search_apps_handler,
    update_app_handler,
};

use super::middleware::{
    auth_middleware, pref_middleware, require_auth_middleware, user_middleware,
};
use super::profile::{profile_controls_handler, profile_page_handler};
use super::users::{
    change_password_handler, delete_user_handler, new_user_handler, post_delete_user_handler,
    post_new_user_handler, post_update_user_status_handler, update_user_status_handler,
    user_controls_handler, user_page_handler, users_handler,
};
use super::{dark_theme_handler, handle_error, light_theme_handler};

pub fn all_routes(state: AppState, frontend_dir: &PathBuf) -> Router {
    Router::new()
        .merge(public_routes(state.clone()))
        .merge(private_routes(state.clone()))
        .merge(assets_routes(frontend_dir))
        .fallback(any(error_handler).with_state(state))
}

pub fn assets_routes(dir: &PathBuf) -> Router {
    let target_dir = dir.join("public");
    Router::new()
        .route(
            "/manifest.json",
            get_service(ServeFile::new(target_dir.join("manifest.json"))),
        )
        .route(
            "/favicon.ico",
            get_service(ServeFile::new(target_dir.join("favicon.ico"))),
        )
        .nest_service(
            "/assets",
            get_service(
                ServeDir::new(target_dir.join("assets"))
                    .not_found_service(file_not_found.into_service()),
            ),
        )
}

async fn file_not_found() -> impl IntoResponse {
    (StatusCode::NOT_FOUND, "File not found")
}

pub fn private_routes(state: AppState) -> Router {
    Router::new()
        .route("/", get(index_handler))
        .route("/prefs/theme/light", post(light_theme_handler))
        .route("/prefs/theme/dark", post(dark_theme_handler))
        .route("/profile", get(profile_page_handler))
        .route("/profile/profile_controls", get(profile_controls_handler))
        .route(
            "/profile/change_password",
            get(change_current_password_handler).post(post_change_current_password_handler),
        )
        .nest("/users", users_routes(state.clone()))
        .nest("/apps", apps_routes(state.clone()))
        // .nest("/clients", client_routes(state.clone()))
        // .nest("/buckets/{bucket_id}", my_bucket_routes(state.clone()))
        .layer(middleware::map_response_with_state(
            state.clone(),
            response_mapper,
        ))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            require_auth_middleware,
        ))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .route_layer(middleware::from_fn(pref_middleware))
        .with_state(state)
}

// fn client_routes(state: AppState) -> Router<AppState> {
//     Router::new()
//         .route("/", get(clients_handler))
//         .route("/listing", get(clients_listing_handler))
//         .route(
//             "/new",
//             get(new_client_handler).post(post_new_client_handler),
//         )
//         .nest("/{client_id}", client_inner_routes(state.clone()))
//         .with_state(state)
// }

// fn client_inner_routes(state: AppState) -> Router<AppState> {
//     Router::new()
//         .route("/", get(client_page_handler))
//         .route("/edit_controls", get(edit_client_controls_handler))
//         .route(
//             "/edit",
//             get(edit_client_handler).post(post_edit_client_handler),
//         )
//         .route(
//             "/delete",
//             get(delete_client_handler).post(post_delete_client_handler),
//         )
//         .nest("/users", users_routes(state.clone()))
//         .nest("/buckets", buckets_routes(state.clone()))
//         .route_layer(middleware::from_fn_with_state(
//             state.clone(),
//             client_middleware,
//         ))
//         .with_state(state)
// }

fn users_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(users_handler))
        .route("/search", get(search_users_handler))
        .route("/new", get(new_user_handler).post(post_new_user_handler))
        .nest("/{user_id}", user_inner_routes(state.clone()))
        .with_state(state)
}

fn user_inner_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(user_page_handler))
        .route("/edit_controls", get(user_controls_handler))
        .route(
            "/update_status",
            get(update_user_status_handler).post(post_update_user_status_handler),
        )
        .route(
            "/change-password",
            get(change_password_handler).post(post_change_password_handler),
        )
        .route(
            "/delete",
            get(delete_user_handler).post(post_delete_user_handler),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            user_middleware,
        ))
        .with_state(state)
}

fn apps_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(apps_handler))
        .route("/search", get(search_apps_handler))
        .route("/new", get(new_app_handler).post(post_new_app_handler))
        .nest("/{app_id}", app_inner_routes(state.clone()))
        .with_state(state)
}

fn app_inner_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(app_page_handler))
        .route("/edit_controls", get(app_controls_handler))
        .route(
            "/edit",
            get(update_app_handler).post(post_update_app_handler),
        )
        .route(
            "/regenerate-secret",
            get(regenerate_app_secret_handler).post(post_regenerate_app_secret_handler),
        )
        .route(
            "/delete",
            get(delete_app_handler).post(post_delete_app_handler),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            app_middleware,
        ))
        .with_state(state)
}

// fn buckets_routes(state: AppState) -> Router<AppState> {
//     Router::new()
//         .route("/", get(buckets_handler))
//         .route(
//             "/new",
//             get(new_bucket_handler).post(post_new_bucket_handler),
//         )
//         .nest("/{bucket_id}", bucket_inner_routes(state.clone()))
//         .with_state(state)
// }

// fn bucket_inner_routes(state: AppState) -> Router<AppState> {
//     Router::new()
//         .route("/", get(bucket_page_handler))
//         .route("/edit_controls", get(bucket_controls_handler))
//         .route(
//             "/edit",
//             get(edit_bucket_handler).post(post_edit_bucket_handler),
//         )
//         .route(
//             "/delete",
//             get(delete_bucket_handler).post(post_delete_bucket_handler),
//         )
//         .route_layer(middleware::from_fn_with_state(
//             state.clone(),
//             bucket_middleware,
//         ))
//         .with_state(state)
// }

// fn my_bucket_routes(state: AppState) -> Router<AppState> {
//     Router::new()
//         .route("/", get(my_bucket_page_handler))
//         .route("/search_dirs", get(search_dirs_handler))
//         .route("/new_dir", get(new_dir_handler).post(post_new_dir_handler))
//         .nest("/dirs/{dir_id}", my_dir_inner_routes(state.clone()))
//         .route_layer(middleware::from_fn_with_state(
//             state.clone(),
//             my_bucket_middleware,
//         ))
//         .with_state(state)
// }

// fn my_dir_inner_routes(state: AppState) -> Router<AppState> {
//     Router::new()
//         .route("/", get(dir_page_handler))
//         .route("/edit_controls", get(edit_dir_controls_handler))
//         .route("/edit", get(edit_dir_handler).post(post_edit_dir_handler))
//         .route(
//             "/delete",
//             get(get_delete_dir_handler).post(post_delete_dir_handler),
//         )
//         .route("/photo_grid", get(photo_listing_v2_handler))
//         .nest("/upload", my_upload_route(state.clone()))
//         .nest("/photos/{file_id}", my_photo_routes(state.clone()))
//         .route_layer(middleware::from_fn_with_state(
//             state.clone(),
//             dir_middleware,
//         ))
//         .with_state(state)
// }

// fn my_upload_route(state: AppState) -> Router<AppState> {
//     Router::new()
//         .route("/", get(upload_page_handler).post(upload_handler))
//         .layer(DefaultBodyLimit::max(8000000))
//         .layer(RequestBodyLimitLayer::new(8000000))
//         .with_state(state)
// }

// fn my_photo_routes(state: AppState) -> Router<AppState> {
//     Router::new()
//         .route(
//             "/delete",
//             get(confirm_delete_photo_handler).post(exec_delete_photo_handler),
//         )
//         .route("/delete_controls", get(pre_delete_photo_handler))
//         .route_layer(middleware::from_fn_with_state(
//             state.clone(),
//             file_middleware,
//         ))
//         .with_state(state)
// }

pub fn public_routes(state: AppState) -> Router {
    Router::new()
        .route("/login", get(login_handler).post(post_login_handler))
        .route("/logout", post(logout_handler))
        .layer(middleware::map_response_with_state(
            state.clone(),
            response_mapper,
        ))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .route_layer(middleware::from_fn(pref_middleware))
        .with_state(state)
}

async fn response_mapper(
    State(state): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    headers: HeaderMap,
    res: Response,
) -> Response {
    let error = res.extensions().get::<ErrorInfo>();
    if let Some(e) = error {
        if e.status_code.is_server_error() {
            error!("{}", e.message);
        }

        let full_page = headers.get("HX-Request").is_none();
        return handle_error(&state, ctx.actor.clone(), &pref, e.clone(), full_page);
    }
    res
}
