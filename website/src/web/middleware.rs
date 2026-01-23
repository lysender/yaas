use axum::{
    Extension,
    extract::{Path, Request, State},
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;
use yaas::dto::Actor;

use crate::{
    Error, Result,
    ctx::Ctx,
    error::ErrorInfo,
    models::{AppParams, OrgAppParams, OrgMemberParams, OrgParams, Pref, UserParams},
    run::AppState,
    services::{
        auth::authenticate_token, get_app_svc, get_org_app_svc, get_org_member_svc, get_org_svc,
        users::get_user_svc,
    },
    web::{Action, Resource, enforce_policy, handle_error},
};

use super::{AUTH_TOKEN_COOKIE, THEME_COOKIE};

/// Validates auth token but does not require its validity
pub async fn auth_middleware(
    pref: Extension<Pref>,
    state: State<AppState>,
    cookies: CookieJar,
    mut req: Request,
    next: Next,
) -> Response {
    let token = cookies
        .get(AUTH_TOKEN_COOKIE)
        .map(|c| c.value().to_string());

    let full_page = req.headers().get("HX-Request").is_none();

    // Allow ctx to be always present
    let mut ctx: Ctx = Ctx::new(Actor::default(), None);

    if let Some(token) = token {
        // Validate token
        let result = authenticate_token(&state, &token).await;

        match result {
            Ok(actor) => {
                ctx = Ctx::new(actor, Some(token));
            }
            Err(err) => match err {
                Error::LoginRequired => {
                    // Allow passing through
                    
                }
                _ => {
                    return handle_error(
                        &state,
                        Actor::default(),
                        &pref,
                        ErrorInfo::from(&err),
                        full_page,
                    );
                }
            },
        };
    }

    req.extensions_mut().insert(ctx);
    next.run(req).await
}

pub async fn require_auth_middleware(
    ctx: Extension<Ctx>,
    req: Request,
    next: Next,
) -> Result<Response> {
    let full_page = req.headers().get("HX-Request").is_none();

    if !ctx.actor.has_auth_scope() {
        if full_page {
            return Ok(Redirect::to("/login").into_response());
        } else {
            return Err(Error::LoginRequired);
        }
    }

    Ok(next.run(req).await)
}

pub async fn user_middleware(
    state: State<AppState>,
    ctx: Extension<Ctx>,
    params: Path<UserParams>,
    mut req: Request,
    next: Next,
) -> Result<Response> {
    enforce_policy(&ctx.actor, Resource::User, Action::Read)?;

    let user = get_user_svc(&state, &ctx, params.user_id).await?;

    req.extensions_mut().insert(user);
    Ok(next.run(req).await)
}

pub async fn app_middleware(
    state: State<AppState>,
    ctx: Extension<Ctx>,
    params: Path<AppParams>,
    mut req: Request,
    next: Next,
) -> Result<Response> {
    enforce_policy(&ctx.actor, Resource::App, Action::Read)?;

    let app = get_app_svc(&state, &ctx, params.app_id).await?;

    req.extensions_mut().insert(app);
    Ok(next.run(req).await)
}

pub async fn org_middleware(
    state: State<AppState>,
    ctx: Extension<Ctx>,
    params: Path<OrgParams>,
    mut req: Request,
    next: Next,
) -> Result<Response> {
    enforce_policy(&ctx.actor, Resource::Org, Action::Read)?;

    let org = get_org_svc(&state, &ctx, params.org_id).await?;

    req.extensions_mut().insert(org);
    Ok(next.run(req).await)
}

pub async fn org_member_middleware(
    state: State<AppState>,
    ctx: Extension<Ctx>,
    params: Path<OrgMemberParams>,
    mut req: Request,
    next: Next,
) -> Result<Response> {
    enforce_policy(&ctx.actor, Resource::OrgMember, Action::Read)?;

    let org_member = get_org_member_svc(&state, &ctx, params.org_id, params.user_id).await?;

    req.extensions_mut().insert(org_member);
    Ok(next.run(req).await)
}

pub async fn org_app_middleware(
    state: State<AppState>,
    ctx: Extension<Ctx>,
    params: Path<OrgAppParams>,
    mut req: Request,
    next: Next,
) -> Result<Response> {
    enforce_policy(&ctx.actor, Resource::OrgApp, Action::Read)?;

    let org_app = get_org_app_svc(&state, &ctx, params.org_id, params.app_id).await?;

    req.extensions_mut().insert(org_app);
    Ok(next.run(req).await)
}

pub async fn pref_middleware(cookies: CookieJar, mut req: Request, next: Next) -> Response {
    let mut pref = Pref::new();
    let theme = cookies.get(THEME_COOKIE).map(|c| c.value().to_string());

    if let Some(theme) = theme {
        let t = theme.as_str();
        if t == "dark" || t == "light" {
            pref.theme = theme;
        }
    }

    req.extensions_mut().insert(pref);
    next.run(req).await
}
