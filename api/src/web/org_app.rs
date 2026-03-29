use axum::{
    Extension, Router,
    body::Body,
    extract::{Query, State},
    http::StatusCode,
    middleware,
    response::Response,
    routing::get,
};
use snafu::{OptionExt, ensure};
use validator::Validate;

use yaas::{
    dto::{Actor, ListOrgAppsParamsDto, NewOrgAppDto, OrgAppDto, OrgDto},
    role::Permission,
    validators::flatten_errors,
};

use crate::{
    Result,
    error::{ForbiddenSnafu, ValidationSnafu, WhateverSnafu},
    services::{
        app::get_app_svc,
        org_app::{create_org_app_svc, delete_org_app_svc, list_org_apps_svc},
    },
    state::AppState,
    web::{
        empty_response,
        json_input::{JsonPayload, parse_and_validate_json},
        json_response,
        middleware::org_app_middleware,
    },
};

pub fn org_apps_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_org_apps_handler).post(create_org_app_handler))
        .nest("/{app_id}", org_apps_inner_routes(state.clone()))
        .with_state(state)
}

fn org_apps_inner_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(get_org_app_handler).delete(delete_org_app_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            org_app_middleware,
        ))
        .with_state(state)
}

async fn list_org_apps_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    org: Extension<OrgDto>,
    query: Query<ListOrgAppsParamsDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::OrgAppsList];
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

    let org_id = org.id.clone();
    let org_apps = list_org_apps_svc(&state, &org_id, query.0).await?;
    Ok(json_response(StatusCode::OK, org_apps))
}

async fn create_org_app_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    org: Extension<OrgDto>,
    payload: JsonPayload<NewOrgAppDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::OrgAppsCreate];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let data = parse_and_validate_json(payload)?;

    let org_id = org.id.clone();
    let mut org_app = create_org_app_svc(&state, &org_id, data).await?;

    // We need to fetch the app name from the app service
    let app = get_app_svc(&state, &org_app.app_id).await?;
    let app = app.context(WhateverSnafu {
        msg: "Unable to fetch app information for org app.",
    })?;

    org_app.app_name = Some(app.name);

    Ok(json_response(StatusCode::CREATED, org_app))
}

async fn get_org_app_handler(
    state: State<AppState>,
    org_app: Extension<OrgAppDto>,
) -> Result<Response<Body>> {
    let org_app = org_app.0;

    // We need to fetch the app name from the app service
    let app = get_app_svc(&state, &org_app.app_id).await?;
    let app = app.context(WhateverSnafu {
        msg: "Unable to fetch app information for org app.",
    })?;

    let mut org_app = org_app;
    org_app.app_name = Some(app.name);
    Ok(json_response(StatusCode::OK, org_app))
}

async fn delete_org_app_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    org_app: Extension<OrgAppDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::OrgAppsDelete];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    delete_org_app_svc(&state, &org_app.id).await?;

    Ok(empty_response(StatusCode::NO_CONTENT))
}
