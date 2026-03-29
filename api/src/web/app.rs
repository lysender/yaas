use axum::{
    Extension, Router,
    body::Body,
    extract::{Query, State},
    http::StatusCode,
    middleware,
    response::Response,
    routing::{get, post},
};
use snafu::{OptionExt, ensure};
use validator::Validate;

use yaas::{
    dto::{Actor, AppDto, ListAppsParamsDto, NewAppDto, UpdateAppDto},
    role::Permission,
    validators::flatten_errors,
};

use crate::{
    Result,
    error::{ForbiddenSnafu, ValidationSnafu, WhateverSnafu},
    services::app::{
        create_app_svc, delete_app_svc, get_app_svc, list_apps_svc, regenerate_app_secret_svc,
        update_app_svc,
    },
    state::AppState,
    web::{
        empty_response,
        json_input::{JsonPayload, parse_and_validate_json},
        json_response,
        middleware::app_middleware,
    },
};

pub fn apps_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_apps_handler).post(create_app_handler))
        .nest("/{app_id}", inner_app_routes(state.clone()))
        .with_state(state)
}

fn inner_app_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(get_app_handler)
                .patch(update_app_handler)
                .delete(delete_app_handler),
        )
        .route("/regenerate-secret", post(regenerate_app_secret_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            app_middleware,
        ))
        .with_state(state)
}

async fn list_apps_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    query: Query<ListAppsParamsDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::AppsList];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let errors = query.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let apps = list_apps_svc(&state, query.0).await?;
    Ok(json_response(StatusCode::OK, apps))
}

async fn create_app_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    payload: JsonPayload<NewAppDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::AppsCreate];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let data = parse_and_validate_json(payload)?;

    let app = create_app_svc(&state, data).await?;
    Ok(json_response(StatusCode::CREATED, app))
}

async fn get_app_handler(app: Extension<AppDto>) -> Result<Response<Body>> {
    Ok(json_response(StatusCode::OK, app.0))
}

async fn update_app_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    app: Extension<AppDto>,
    payload: JsonPayload<UpdateAppDto>,
) -> Result<Response<Body>> {
    let app = app.0;
    let app_id = app.id;

    let permissions = vec![Permission::AppsEdit];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let data = parse_and_validate_json(payload)?;

    let _ = update_app_svc(&state, &app_id, data).await?;

    // Not ideal but we need to re-query to get the updated data
    let updated_app = get_app_svc(&state, &app_id).await?;
    let updated_app = updated_app.context(WhateverSnafu {
        msg: "Unable to re-query app information.",
    })?;

    Ok(json_response(StatusCode::OK, updated_app))
}

async fn regenerate_app_secret_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    app: Extension<AppDto>,
) -> Result<Response<Body>> {
    let app = app.0;
    let app_id = app.id;

    let permissions = vec![Permission::AppsEdit];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let _ = regenerate_app_secret_svc(&state, &app_id).await?;

    // Not ideal but we need to re-query to get the updated data
    let updated_app = get_app_svc(&state, &app_id).await?;
    let updated_app = updated_app.context(WhateverSnafu {
        msg: "Unable to re-query app information.",
    })?;

    Ok(json_response(StatusCode::OK, updated_app))
}

async fn delete_app_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    app: Extension<AppDto>,
) -> Result<Response<Body>> {
    let app = app.0;
    let app_id = app.id;

    let permissions = vec![Permission::AppsDelete];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let _ = delete_app_svc(&state, &app_id).await?;

    Ok(empty_response(StatusCode::NO_CONTENT))
}
