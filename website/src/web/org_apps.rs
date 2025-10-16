use askama::Template;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::{Extension, Form, body::Body, extract::State, response::Response};
use axum::{Router, middleware, routing::get};
use snafu::{ResultExt, ensure};
use urlencoding::encode;
use validator::Validate;

use crate::error::ValidationSnafu;
use crate::models::{PaginationLinks, UserParams};
use crate::services::users::{get_user_svc, list_users_svc};
use crate::services::{
    UpdateOrgFormData, UpdateOrgOwnerFormData, create_org_svc, get_org_member_svc,
    list_org_members_svc, list_orgs_svc, update_org_owner_svc, update_org_svc,
};
use crate::web::middleware::{org_app_middleware, org_middleware};
use crate::web::org_members_routes;
use crate::{
    Error, Result,
    ctx::Ctx,
    error::{ErrorInfo, ResponseBuilderSnafu, TemplateSnafu},
    models::{Pref, TemplateData},
    run::AppState,
    services::{NewOrgFormData, token::create_csrf_token_svc},
    web::{Action, Resource, enforce_policy},
};
use yaas::dto::{ListOrgMembersParamsDto, ListOrgsParamsDto, OrgMemberDto, UserDto};
use yaas::dto::{ListUsersParamsDto, OrgDto};
use yaas::role::Permission;
use yaas::validators::flatten_errors;

pub fn org_apps_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(orgs_handler))
        .route("/new", get(orgs_handler).post(orgs_handler))
        .nest("/{app_id}", org_app_inner_routes(state.clone()))
        .with_state(state)
}

fn org_app_inner_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(orgs_handler))
        .route("/edit-controls", get(orgs_handler))
        .route("/edit", get(orgs_handler).post(orgs_handler))
        .route("/change-owner", get(orgs_handler).post(orgs_handler))
        .route("/search-owner", get(orgs_handler))
        .route("/select-owner/{user_id}", get(orgs_handler))
        // .route(
        //     "/delete",
        //     get(delete_user_handler).post(post_delete_user_handler),
        // )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            org_app_middleware,
        ))
        .with_state(state)
}

#[derive(Template)]
#[template(path = "pages/orgs/index.html")]
struct OrgsPageTemplate {
    t: TemplateData,
    query_params: String,
}

async fn orgs_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    State(state): State<AppState>,
    Query(query): Query<ListOrgsParamsDto>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::Org, Action::Read)?;

    let errors = query.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);
    t.title = String::from("Orgs");

    let tpl = OrgsPageTemplate {
        t,
        query_params: query.to_string(),
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}
