use askama::Template;
use axum::debug_handler;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::{Extension, Form, body::Body, extract::State, response::Response};
use snafu::{ResultExt, ensure};
use urlencoding::encode;
use validator::Validate;
use yaas::validators::flatten_errors;

use crate::error::ValidationSnafu;
use crate::models::{PaginationLinks, TokenFormData};
use crate::services::{
    NewAppFormData, UpdateAppFormData, create_app_svc, delete_app_svc, list_apps_svc,
    regenerate_app_secret_svc, update_app_svc,
};
use crate::{
    Error, Result,
    ctx::Ctx,
    error::{ErrorInfo, ResponseBuilderSnafu, TemplateSnafu},
    models::{Pref, TemplateData},
    run::AppState,
    services::token::create_csrf_token_svc,
    web::{Action, Resource, enforce_policy},
};
use yaas::dto::AppDto;
use yaas::dto::ListAppsParamsDto;
use yaas::role::Permission;

#[derive(Template)]
#[template(path = "pages/apps/index.html")]
struct AppsPageTemplate {
    t: TemplateData,
    query_params: String,
}

pub async fn org_members_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    State(state): State<AppState>,
    Query(query): Query<ListAppsParamsDto>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::App, Action::Read)?;

    let errors = query.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);
    t.title = String::from("Apps");

    let tpl = AppsPageTemplate {
        t,
        query_params: query.to_string(),
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/apps/search.html")]
struct SearchAppsTemplate {
    apps: Vec<AppDto>,
    pagination: Option<PaginationLinks>,
    error_message: Option<String>,
}
pub async fn search_org_members_handler(
    Extension(ctx): Extension<Ctx>,
    State(state): State<AppState>,
    Query(query): Query<ListAppsParamsDto>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::App, Action::Read)?;

    let mut tpl = SearchAppsTemplate {
        apps: Vec::new(),
        pagination: None,
        error_message: None,
    };

    let keyword = query.keyword.clone();

    match list_apps_svc(&state, &ctx, query).await {
        Ok(apps) => {
            let mut keyword_param: String = "".to_string();
            if let Some(keyword) = &keyword {
                keyword_param = format!("&keyword={}", encode(keyword).to_string());
            }
            tpl.apps = apps.data;
            tpl.pagination = Some(PaginationLinks::new(
                &apps.meta,
                "/apps/search",
                "/apps",
                &keyword_param,
                ".album-items",
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
#[template(path = "pages/apps/new.html")]
struct NewAppTemplate {
    t: TemplateData,
    action: String,
    payload: NewAppFormData,
    error_message: Option<String>,
}

#[derive(Template)]
#[template(path = "widgets/apps/new_form.html")]
struct NewAppFormTemplate {
    action: String,
    payload: NewAppFormData,
    error_message: Option<String>,
}

pub async fn new_org_member_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::App, Action::Create)?;

    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);
    t.title = String::from("Create New App");

    let token = create_csrf_token_svc("new_app", &config.jwt_secret)?;

    let tpl = NewAppTemplate {
        t,
        action: "/apps/new".to_string(),
        payload: NewAppFormData {
            name: "".to_string(),
            redirect_uri: "".to_string(),
            token,
        },
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

pub async fn post_new_org_member_handler(
    Extension(ctx): Extension<Ctx>,
    State(state): State<AppState>,
    Form(payload): Form<NewAppFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::App, Action::Create)?;

    let token = create_csrf_token_svc("new_app", &config.jwt_secret)?;

    let mut tpl = NewAppFormTemplate {
        action: "/apps/new".to_string(),
        payload: NewAppFormData {
            name: "".to_string(),
            redirect_uri: "".to_string(),
            token,
        },
        error_message: None,
    };

    let status: StatusCode;

    let app = NewAppFormData {
        name: payload.name.clone(),
        redirect_uri: payload.redirect_uri.clone(),
        token: payload.token.clone(),
    };

    let result = create_app_svc(&state, &ctx, app).await;

    match result {
        Ok(_) => {
            let next_url = "/apps".to_string();
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

    tpl.payload.name = payload.name.clone();
    tpl.payload.redirect_uri = payload.redirect_uri.clone();

    // Will only arrive here on error
    Ok(Response::builder()
        .status(status)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "pages/apps/view.html")]
struct AppPageTemplate {
    t: TemplateData,
    app: AppDto,
    updated: bool,
    can_edit: bool,
    can_delete: bool,
}

pub async fn org_member_page_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Extension(app): Extension<AppDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);

    t.title = format!("App - {}", &app.name);

    let tpl = AppPageTemplate {
        t,
        app,
        updated: false,
        can_edit: ctx.actor.has_permissions(&vec![Permission::AppsEdit]),
        can_delete: ctx.actor.has_permissions(&vec![Permission::AppsDelete]),
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/apps/edit_controls.html")]
struct AppControlsTemplate {
    app: AppDto,
    updated: bool,
    can_edit: bool,
    can_delete: bool,
}

pub async fn org_member_controls_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(app): Extension<AppDto>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::App, Action::Update)?;

    let tpl = AppControlsTemplate {
        app,
        updated: false,
        can_edit: ctx.actor.has_permissions(&vec![Permission::AppsEdit]),
        can_delete: ctx.actor.has_permissions(&vec![Permission::AppsDelete]),
    };

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/apps/update_form.html")]
struct UpdateAppTemplate {
    app: AppDto,
    payload: UpdateAppFormData,
    error_message: Option<String>,
}

pub async fn update_org_member_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(app): Extension<AppDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::App, Action::Update)?;
    let token = create_csrf_token_svc(app.id.to_string().as_str(), &config.jwt_secret)?;

    let name = app.name.clone();
    let redirect_uri = app.redirect_uri.clone();

    let tpl = UpdateAppTemplate {
        app,
        payload: UpdateAppFormData {
            token,
            name,
            redirect_uri,
        },
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[debug_handler]
pub async fn post_update_org_member_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(app): Extension<AppDto>,
    State(state): State<AppState>,
    payload: Form<UpdateAppFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::App, Action::Update)?;

    let token = create_csrf_token_svc(&app.id.to_string(), &config.jwt_secret)?;
    let app_id = app.id;

    let mut tpl = UpdateAppTemplate {
        app,
        payload: UpdateAppFormData {
            token,
            name: payload.name.clone(),
            redirect_uri: payload.redirect_uri.clone(),
        },
        error_message: None,
    };

    let data = UpdateAppFormData {
        token: payload.token.clone(),
        name: payload.name.clone(),
        redirect_uri: payload.redirect_uri.clone(),
    };

    let result = update_app_svc(&state, &ctx, app_id, data).await;

    match result {
        Ok(updated_app) => {
            // Render back the controls but with updated data
            let tpl = AppControlsTemplate {
                app: updated_app,
                updated: true,
                can_edit: ctx.actor.has_permissions(&vec![Permission::AppsEdit]),
                can_delete: ctx.actor.has_permissions(&vec![Permission::AppsDelete]),
            };

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "text/html")
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
        Err(err) => {
            let status;
            match err {
                Error::Validation { msg } => {
                    status = StatusCode::BAD_REQUEST;
                    tpl.error_message = Some(msg);
                }
                Error::LoginRequired => {
                    status = StatusCode::UNAUTHORIZED;
                    tpl.error_message = Some("Login required.".to_string());
                }
                any_err => {
                    status = StatusCode::INTERNAL_SERVER_ERROR;
                    tpl.error_message = Some(any_err.to_string());
                }
            };

            Ok(Response::builder()
                .status(status)
                .header("Content-Type", "text/html")
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
    }
}

#[derive(Template)]
#[template(path = "widgets/apps/delete_form.html")]
struct DeleteAppFormTemplate {
    app: AppDto,
    payload: TokenFormData,
    error_message: Option<String>,
}

pub async fn delete_org_member_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(app): Extension<AppDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::App, Action::Delete)?;

    let token = create_csrf_token_svc(&app.id.to_string(), &config.jwt_secret)?;

    let tpl = DeleteAppFormTemplate {
        app,
        payload: TokenFormData { token },
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

pub async fn post_delete_org_member_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(app): Extension<AppDto>,
    State(state): State<AppState>,
    payload: Form<TokenFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::App, Action::Delete)?;

    let token = create_csrf_token_svc(&app.id.to_string(), &config.jwt_secret)?;

    let mut tpl = DeleteAppFormTemplate {
        app: app.clone(),
        payload: TokenFormData { token },
        error_message: None,
    };

    let result = delete_app_svc(&state, &ctx, app.id, &payload.token).await;

    match result {
        Ok(_) => {
            // Render same form but trigger a redirect to home
            let tpl = DeleteAppFormTemplate {
                app,
                payload: TokenFormData {
                    token: "".to_string(),
                },
                error_message: None,
            };
            return Ok(Response::builder()
                .status(200)
                .header("HX-Redirect", "/apps".to_string())
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
