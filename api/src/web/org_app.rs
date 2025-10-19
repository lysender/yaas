use axum::{
    Extension, Router,
    body::{Body, Bytes},
    extract::{Query, State},
    middleware,
    response::Response,
    routing::get,
};
use prost::Message;
use snafu::{OptionExt, ensure};
use validator::Validate;

use yaas::{
    buffed::{
        dto::{NewOrgAppBuf, OrgAppBuf, PaginatedOrgAppsBuf},
        pagination::PaginatedMetaBuf,
    },
    dto::{Actor, ListOrgAppsParamsDto, NewOrgAppDto, OrgAppDto, OrgDto},
    role::Permission,
    validators::flatten_errors,
};

use crate::{
    Error, Result,
    error::{ForbiddenSnafu, ValidationSnafu, WhateverSnafu},
    services::{
        app::get_app_svc,
        org_app::{create_org_app_svc, delete_org_app_svc, list_org_apps_svc},
    },
    state::AppState,
    web::{build_response, middleware::org_app_middleware},
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

    let org_apps = list_org_apps_svc(&state, org.id, query.0).await?;
    let buffed_meta = PaginatedMetaBuf {
        page: org_apps.meta.page,
        per_page: org_apps.meta.per_page,
        total_records: org_apps.meta.total_records,
        total_pages: org_apps.meta.total_pages,
    };
    let buffed_list: Vec<OrgAppBuf> = org_apps
        .data
        .into_iter()
        .map(|org_app| OrgAppBuf {
            id: org_app.id,
            org_id: org_app.org_id,
            app_id: org_app.app_id,
            app_name: org_app.app_name,
            created_at: org_app.created_at,
        })
        .collect();

    let buffed_result = PaginatedOrgAppsBuf {
        meta: Some(buffed_meta),
        data: buffed_list,
    };

    Ok(build_response(200, buffed_result.encode_to_vec()))
}

async fn create_org_app_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    org: Extension<OrgDto>,
    body: Bytes,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::OrgAppsCreate];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    // Parse body as protobuf message
    let Ok(payload) = NewOrgAppBuf::decode(body) else {
        return Err(Error::BadProtobuf);
    };

    let data: NewOrgAppDto = payload.into();
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let mut org_app = create_org_app_svc(&state, org.id, data).await?;

    // We need to fetch the app name from the app service
    let app = get_app_svc(&state, org_app.app_id).await?;
    let app = app.context(WhateverSnafu {
        msg: "Unable to fetch app information for org app.",
    })?;

    org_app.app_name = Some(app.name);

    let buffed_org_app = OrgAppBuf {
        id: org_app.id,
        org_id: org_app.org_id,
        app_id: org_app.app_id,
        app_name: org_app.app_name,
        created_at: org_app.created_at,
    };

    Ok(build_response(201, buffed_org_app.encode_to_vec()))
}

async fn get_org_app_handler(
    state: State<AppState>,
    org_app: Extension<OrgAppDto>,
) -> Result<Response<Body>> {
    // We need to fetch the app name from the app service
    let app = get_app_svc(&state, org_app.app_id).await?;
    let app = app.context(WhateverSnafu {
        msg: "Unable to fetch app information for org app.",
    })?;

    let buffed_org_app = OrgAppBuf {
        id: org_app.id,
        org_id: org_app.org_id,
        app_id: org_app.app_id,
        app_name: Some(app.name),
        created_at: org_app.created_at.clone(),
    };

    Ok(build_response(200, buffed_org_app.encode_to_vec()))
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

    let _ = delete_org_app_svc(&state, org_app.id).await?;

    Ok(build_response(204, Vec::new()))
}
