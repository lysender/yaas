use axum::{
    Extension, Router,
    body::{Body, Bytes},
    extract::{Query, State},
    middleware,
    response::Response,
    routing::{get, post},
};
use prost::Message;
use snafu::{OptionExt, ensure};
use validator::Validate;

use yaas::{
    buffed::{
        dto::{AppBuf, NewAppBuf, PaginatedAppsBuf, UpdateAppBuf},
        pagination::PaginatedMetaBuf,
    },
    dto::{Actor, AppDto, ListAppsParamsDto, NewAppDto, UpdateAppDto},
    role::Permission,
    validators::flatten_errors,
};

use crate::{
    Error, Result,
    error::{ForbiddenSnafu, ValidationSnafu, WhateverSnafu},
    services::app::{
        create_app_svc, delete_app_svc, get_app_svc, list_apps_svc, regenerate_app_secret_svc,
        update_app_svc,
    },
    state::AppState,
    web::{build_response, middleware::app_middleware},
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
    let buffed_meta = PaginatedMetaBuf {
        page: apps.meta.page,
        per_page: apps.meta.per_page,
        total_records: apps.meta.total_records,
        total_pages: apps.meta.total_pages,
    };
    let buffed_list: Vec<AppBuf> = apps
        .data
        .into_iter()
        .map(|app| AppBuf {
            id: app.id,
            name: app.name,
            client_id: app.client_id,
            client_secret: app.client_secret,
            redirect_uri: app.redirect_uri,
            created_at: app.created_at,
            updated_at: app.updated_at,
        })
        .collect();

    let buffed_result = PaginatedAppsBuf {
        meta: Some(buffed_meta),
        data: buffed_list,
    };

    Ok(build_response(200, buffed_result.encode_to_vec()))
}

async fn create_app_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    body: Bytes,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::AppsCreate];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    // Parse body as protobuf message
    let Ok(payload) = NewAppBuf::decode(body) else {
        return Err(Error::BadProtobuf);
    };

    let data: NewAppDto = payload.into();
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let app = create_app_svc(&state, data).await?;

    let buffed_app = AppBuf {
        id: app.id,
        name: app.name,
        client_id: app.client_id,
        client_secret: app.client_secret,
        redirect_uri: app.redirect_uri,
        created_at: app.created_at,
        updated_at: app.updated_at,
    };

    Ok(build_response(201, buffed_app.encode_to_vec()))
}

async fn get_app_handler(app: Extension<AppDto>) -> Result<Response<Body>> {
    let buffed_app = AppBuf {
        id: app.id,
        name: app.name.clone(),
        client_id: app.client_id.clone(),
        client_secret: app.client_secret.clone(),
        redirect_uri: app.redirect_uri.clone(),
        created_at: app.created_at.clone(),
        updated_at: app.updated_at.clone(),
    };

    Ok(build_response(200, buffed_app.encode_to_vec()))
}

async fn update_app_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    app: Extension<AppDto>,
    body: Bytes,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::AppsEdit];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    // Parse body as protobuf message
    let Ok(payload) = UpdateAppBuf::decode(body) else {
        return Err(Error::BadProtobuf);
    };

    let data: UpdateAppDto = payload.into();
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let _ = update_app_svc(&state, app.id, data).await?;

    // Not ideal but we need to re-query to get the updated data
    let updated_app = get_app_svc(&state, app.id).await?;
    let updated_app = updated_app.context(WhateverSnafu {
        msg: "Unable to re-query app information.",
    })?;

    let buffed_app = AppBuf {
        id: updated_app.id,
        name: updated_app.name,
        client_id: updated_app.client_id,
        client_secret: updated_app.client_secret,
        redirect_uri: updated_app.redirect_uri,
        created_at: updated_app.created_at,
        updated_at: updated_app.updated_at,
    };

    Ok(build_response(200, buffed_app.encode_to_vec()))
}

async fn regenerate_app_secret_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    app: Extension<AppDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::AppsEdit];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let _ = regenerate_app_secret_svc(&state, app.id).await?;

    // Not ideal but we need to re-query to get the updated data
    let updated_app = get_app_svc(&state, app.id).await?;
    let updated_app = updated_app.context(WhateverSnafu {
        msg: "Unable to re-query app information.",
    })?;

    let buffed_app = AppBuf {
        id: updated_app.id,
        name: updated_app.name,
        client_id: updated_app.client_id,
        client_secret: updated_app.client_secret,
        redirect_uri: updated_app.redirect_uri,
        created_at: updated_app.created_at,
        updated_at: updated_app.updated_at,
    };

    Ok(build_response(200, buffed_app.encode_to_vec()))
}

async fn delete_app_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    app: Extension<AppDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::AppsDelete];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let _ = delete_app_svc(&state, app.id).await?;

    Ok(build_response(204, Vec::new()))
}
