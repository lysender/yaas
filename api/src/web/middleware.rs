use axum::{
    Extension,
    body::Body,
    extract::{Path, Request, State},
    http::header,
    middleware::Next,
    response::Response,
};
use snafu::{OptionExt, ResultExt, ensure};
use yaas::actor::Actor;

use crate::{
    Result,
    auth::authenticate_token,
    error::{
        DbSnafu, InsufficientAuthScopeSnafu, InvalidAuthTokenSnafu, NotFoundSnafu, WhateverSnafu,
    },
    services::app::get_app_svc,
    state::AppState,
    web::params::{AppParams, OrgAppParams, OrgMemberParams, OrgParams, UserParams},
};

pub async fn auth_middleware(
    state: State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response<Body>> {
    // Middleware to extract actor information from the request
    // Do not enforce authentication here, just extract the actor information
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    // Start with an empty actor
    let mut actor: Actor = Actor::default();

    if let Some(auth_header) = auth_header {
        // At this point, authentication must be verified
        ensure!(auth_header.starts_with("Bearer "), InvalidAuthTokenSnafu);
        let token = auth_header.replace("Bearer ", "");

        actor = authenticate_token(&state, &token).await?;

        ensure!(actor.has_auth_scope(), InvalidAuthTokenSnafu);
    }

    // Forward to the next middleware/handler passing the actor information
    request.extensions_mut().insert(actor);

    let response = next.run(request).await;
    Ok(response)
}

pub async fn require_auth_middleware(
    actor: Extension<Actor>,
    request: Request,
    next: Next,
) -> Result<Response<Body>> {
    ensure!(actor.has_auth_scope(), InsufficientAuthScopeSnafu);

    Ok(next.run(request).await)
}

pub async fn user_middleware(
    state: State<AppState>,
    params: Path<UserParams>,
    mut request: Request,
    next: Next,
) -> Result<Response<Body>> {
    let doc = state.db.users.get(params.user_id).await.context(DbSnafu)?;
    let doc = doc.context(NotFoundSnafu {
        msg: "User not found",
    })?;

    // Forward to the next middleware/handler passing the user information
    request.extensions_mut().insert(doc);
    let response = next.run(request).await;
    Ok(response)
}

pub async fn app_middleware(
    state: State<AppState>,
    params: Path<AppParams>,
    mut request: Request,
    next: Next,
) -> Result<Response<Body>> {
    let doc = state.db.apps.get(params.app_id).await.context(DbSnafu)?;
    let doc = doc.context(NotFoundSnafu {
        msg: "App not found",
    })?;

    // Forward to the next middleware/handler passing the app information
    request.extensions_mut().insert(doc);
    let response = next.run(request).await;
    Ok(response)
}

pub async fn org_middleware(
    state: State<AppState>,
    params: Path<OrgParams>,
    mut request: Request,
    next: Next,
) -> Result<Response<Body>> {
    let doc = state.db.orgs.get(params.org_id).await.context(DbSnafu)?;
    let doc = doc.context(NotFoundSnafu {
        msg: "Org not found",
    })?;

    // Forward to the next middleware/handler passing the org information
    request.extensions_mut().insert(doc);
    let response = next.run(request).await;
    Ok(response)
}

pub async fn org_member_middleware(
    state: State<AppState>,
    params: Path<OrgMemberParams>,
    mut request: Request,
    next: Next,
) -> Result<Response<Body>> {
    let doc = state
        .db
        .org_members
        .get(params.org_member_id)
        .await
        .context(DbSnafu)?;

    let doc = doc.context(NotFoundSnafu {
        msg: "Org member not found",
    })?;

    // Forward to the next middleware/handler passing the org member information
    request.extensions_mut().insert(doc);
    let response = next.run(request).await;
    Ok(response)
}

pub async fn org_app_middleware(
    state: State<AppState>,
    params: Path<OrgAppParams>,
    mut request: Request,
    next: Next,
) -> Result<Response<Body>> {
    let doc = state
        .db
        .org_apps
        .get(params.org_app_id)
        .await
        .context(DbSnafu)?;

    let mut doc = doc.context(NotFoundSnafu {
        msg: "Org app not found",
    })?;

    // We need to fetch the app name from the app service
    let app = get_app_svc(&state, doc.app_id).await?;
    let app = app.context(WhateverSnafu {
        msg: "Unable to fetch app information for org app.",
    })?;

    doc.app_name = Some(app.name);

    // Forward to the next middleware/handler passing the org member information
    request.extensions_mut().insert(doc);
    let response = next.run(request).await;
    Ok(response)
}
