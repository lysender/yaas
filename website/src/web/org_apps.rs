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
    list_org_apps_svc, list_org_members_svc, list_orgs_svc, update_org_owner_svc, update_org_svc,
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
use yaas::dto::{
    ListOrgAppsParamsDto, ListOrgMembersParamsDto, ListOrgsParamsDto, OrgAppDto, OrgMemberDto,
    UserDto,
};
use yaas::dto::{ListUsersParamsDto, OrgDto};
use yaas::role::Permission;
use yaas::validators::flatten_errors;

pub fn org_apps_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(org_apps_handler))
        .route("/search", get(search_org_apps_handler))
        .route("/new", get(org_apps_handler).post(org_apps_handler))
        .nest("/{app_id}", org_app_inner_routes(state.clone()))
        .with_state(state)
}

fn org_app_inner_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(org_apps_handler))
        .route("/edit-controls", get(org_apps_handler))
        .route("/edit", get(org_apps_handler).post(org_apps_handler))
        .route(
            "/change-owner",
            get(org_apps_handler).post(org_apps_handler),
        )
        .route("/search-owner", get(org_apps_handler))
        .route("/select-owner/{user_id}", get(org_apps_handler))
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
#[template(path = "pages/org_apps/index.html")]
struct OrgAppsPageTemplate {
    t: TemplateData,
    org: OrgDto,
    query_params: String,
}

async fn org_apps_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
    Query(query): Query<ListOrgAppsParamsDto>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::OrgApp, Action::Read)?;

    let errors = query.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);
    t.title = String::from("Org Apps");

    let tpl = OrgAppsPageTemplate {
        t,
        org,
        query_params: query.to_string(),
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/org_apps/search.html")]
struct SearchOrgAppsTemplate {
    org_apps: Vec<OrgAppDto>,
    pagination: Option<PaginationLinks>,
    error_message: Option<String>,
}
async fn search_org_apps_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
    Query(query): Query<ListOrgAppsParamsDto>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::OrgMember, Action::Read)?;

    let mut tpl = SearchOrgAppsTemplate {
        org_apps: Vec::new(),
        pagination: None,
        error_message: None,
    };

    let keyword = query.keyword.clone();

    match list_org_apps_svc(&state, &ctx, org.id, query).await {
        Ok(org_apps) => {
            let mut keyword_param: String = "".to_string();
            if let Some(keyword) = &keyword {
                keyword_param = format!("&keyword={}", encode(keyword).to_string());
            }
            tpl.org_apps = org_apps.data;
            tpl.pagination = Some(PaginationLinks::new(
                &org_apps.meta,
                format!("/orgs/{}/apps/search", org.id).as_str(),
                format!("/orgs/{}/apps", org.id).as_str(),
                &keyword_param,
                ".org-apps",
            ));

            Ok(Response::builder()
                .status(200)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
        Err(err) => {
            let error_info = ErrorInfo::from(&err);
            tpl.error_message = Some(error_info.message);

            Ok(Response::builder()
                .status(error_info.status_code)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
    }
}
