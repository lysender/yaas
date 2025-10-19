use askama::Template;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::{Extension, Form, body::Body, extract::State, response::Response};
use axum::{Router, middleware, routing::get};
use snafu::{ResultExt, ensure};
use urlencoding::encode;
use validator::Validate;

use crate::error::ValidationSnafu;
use crate::models::{OrgAppParams, PaginationLinks, TokenFormData};
use crate::services::{
    NewOrgAppFormData, create_org_app_svc, delete_org_app_svc, get_app_svc,
    list_org_app_suggestions_svc, list_org_apps_svc,
};
use crate::web::middleware::org_app_middleware;
use crate::{
    Result,
    ctx::Ctx,
    error::{ErrorInfo, ResponseBuilderSnafu, TemplateSnafu},
    models::{Pref, TemplateData},
    run::AppState,
    services::token::create_csrf_token_svc,
    web::{Action, Resource, enforce_policy},
};
use yaas::dto::OrgDto;
use yaas::dto::{ListOrgAppsParamsDto, OrgAppDto, OrgAppSuggestionDto};
use yaas::role::Permission;
use yaas::validators::flatten_errors;

pub fn org_apps_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(org_apps_handler))
        .route("/search", get(search_org_apps_handler))
        .route(
            "/new",
            get(new_org_app_handler).post(post_new_org_app_handler),
        )
        .route("/app-suggestions", get(search_app_suggestions_handler))
        .route(
            "/select-app-suggestion/{app_id}",
            get(select_org_app_suggestion_handler),
        )
        .nest("/{app_id}", org_app_inner_routes(state.clone()))
        .with_state(state)
}

fn org_app_inner_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(org_app_page_handler))
        .route("/edit-controls", get(org_app_controls_handler))
        .route(
            "/delete",
            get(delete_org_app_handler).post(post_delete_org_app_handler),
        )
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

#[derive(Template)]
#[template(path = "widgets/org_apps/search_app_suggestions.html")]
struct SearchAppSuggestionsTemplate {
    org: OrgDto,
    suggestions: Vec<OrgAppSuggestionDto>,
    error_message: Option<String>,
}

async fn search_app_suggestions_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
    Query(query): Query<ListOrgAppsParamsDto>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::App, Action::Read)?;

    let org_id = org.id;
    let mut tpl = SearchAppSuggestionsTemplate {
        org,
        suggestions: Vec::new(),
        error_message: None,
    };

    let mut updated_query = query.clone();
    updated_query.per_page = Some(10);

    match list_org_app_suggestions_svc(&state, &ctx, org_id, updated_query).await {
        Ok(users) => {
            tpl.suggestions = users.data;

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

#[derive(Template)]
#[template(path = "widgets/org_apps/select_app_suggestion.html")]
struct SelectAppSuggestionTemplate {
    org: OrgDto,
    payload: NewOrgAppFormData,
    error_message: Option<String>,
}

async fn select_org_app_suggestion_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
    Path(params): Path<OrgAppParams>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::App, Action::Read)?;
    let token = create_csrf_token_svc("new_org_app", &state.config.jwt_secret)?;

    let mut tpl = SelectAppSuggestionTemplate {
        org,
        payload: NewOrgAppFormData {
            token,
            app_id: 0,
            app_name: "".to_string(),
        },
        error_message: None,
    };

    match get_app_svc(&state, &ctx, params.app_id).await {
        Ok(app) => {
            tpl.payload.app_id = app.id;
            tpl.payload.app_name = app.name;

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

#[derive(Template)]
#[template(path = "pages/org_apps/new.html")]
struct NewOrgAppTemplate {
    t: TemplateData,
    action: String,
    org: OrgDto,
    error_message: Option<String>,
}

#[derive(Template)]
#[template(path = "widgets/org_apps/new_form.html")]
struct NewOrgAppFormTemplate {
    action: String,
    org: OrgDto,
    payload: NewOrgAppFormData,
    error_message: Option<String>,
}

async fn new_org_app_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::OrgApp, Action::Create)?;

    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);
    t.title = String::from("Add New Org App");

    let tpl = NewOrgAppTemplate {
        t,
        action: format!("/orgs/{}/apps/new", org.id),
        org,
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

async fn post_new_org_app_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
    Form(payload): Form<NewOrgAppFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::OrgApp, Action::Create)?;

    let org_id = org.id;
    let token = create_csrf_token_svc("new_org_app", &config.jwt_secret)?;

    let mut tpl = NewOrgAppFormTemplate {
        action: format!("/orgs/{}/apps/new", org_id),
        org,
        payload: NewOrgAppFormData {
            token,
            app_id: payload.app_id,
            app_name: payload.app_name.clone(),
        },
        error_message: None,
    };

    let status: StatusCode;

    let result = create_org_app_svc(&state, &ctx, org_id, payload).await;

    match result {
        Ok(_) => {
            let next_url = format!("/orgs/{}/apps", org_id);
            // Weird but can't do a redirect here, let htmx handle it
            return Ok(Response::builder()
                .status(200)
                .header("HX-Redirect", next_url)
                .body(Body::from("".to_string()))
                .context(ResponseBuilderSnafu)?);
        }
        Err(err) => {
            let error_info = ErrorInfo::from(&err);
            status = error_info.status_code;
            tpl.error_message = Some(error_info.message);
        }
    }

    // Will only arrive here on error
    Ok(Response::builder()
        .status(status)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "pages/org_apps/view.html")]
struct OrgAppPageTemplate {
    t: TemplateData,
    org: OrgDto,
    org_app: OrgAppDto,
    can_delete: bool,
}

async fn org_app_page_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Extension(org): Extension<OrgDto>,
    Extension(org_app): Extension<OrgAppDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);
    let app_name = org_app.app_name.clone().unwrap_or("".to_string());

    t.title = format!("Org App - {}", app_name,);

    let tpl = OrgAppPageTemplate {
        t,
        org,
        org_app,
        can_delete: ctx.actor.has_permissions(&vec![Permission::OrgAppsDelete]),
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/org_apps/edit_controls.html")]
struct OrgAppControlsTemplate {
    org_app: OrgAppDto,
    can_delete: bool,
}

async fn org_app_controls_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org_app): Extension<OrgAppDto>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::OrgApp, Action::Update)?;

    let tpl = OrgAppControlsTemplate {
        org_app,
        can_delete: ctx.actor.has_permissions(&vec![Permission::OrgAppsDelete]),
    };

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/org_apps/delete_form.html")]
struct DeleteOrgAppFormTemplate {
    org_app: OrgAppDto,
    payload: TokenFormData,
    error_message: Option<String>,
}

async fn delete_org_app_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org_app): Extension<OrgAppDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::OrgApp, Action::Delete)?;

    let token = create_csrf_token_svc(&org_app.app_id.to_string(), &config.jwt_secret)?;

    let tpl = DeleteOrgAppFormTemplate {
        org_app,
        payload: TokenFormData { token },
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

async fn post_delete_org_app_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    Extension(org_app): Extension<OrgAppDto>,
    State(state): State<AppState>,
    payload: Form<TokenFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::OrgApp, Action::Delete)?;

    let token = create_csrf_token_svc(&org_app.app_id.to_string(), &config.jwt_secret)?;
    let org_id = org.id;
    let app_id = org_app.app_id;

    let mut tpl = DeleteOrgAppFormTemplate {
        org_app,
        payload: TokenFormData { token },
        error_message: None,
    };

    let result = delete_org_app_svc(&state, &ctx, org_id, app_id, &payload.token).await;

    match result {
        Ok(_) => {
            // Render same form but trigger a redirect to home
            return Ok(Response::builder()
                .status(200)
                .header("HX-Redirect", format!("/orgs/{}/apps", org_id))
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?);
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
