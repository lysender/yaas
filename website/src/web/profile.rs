use askama::Template;
use axum::extract::Query;
use axum::http::StatusCode;
use axum::{
    Extension, Form,
    body::Body,
    extract::State,
    response::{IntoResponse, Redirect, Response},
};
use axum::{Router, routing::get};
use snafu::ResultExt;
use tower_cookies::{Cookie, Cookies, cookie::time::Duration};
use urlencoding::encode;

use crate::error::ErrorInfo;
use crate::models::PaginationLinks;
use crate::services::auth::{
    SwitchAuthContextFormData, SwitchAuthContextParams, switch_auth_context_svc,
};
use crate::services::users::{
    ChangeCurrentPasswordFormData, change_user_current_password_svc, list_org_memberships_svc,
};
use crate::web::AUTH_TOKEN_COOKIE;
use crate::{
    Error, Result,
    ctx::Ctx,
    error::{ResponseBuilderSnafu, TemplateSnafu},
    models::{Pref, TemplateData},
    run::AppState,
    services::token::create_csrf_token_svc,
};
use yaas::dto::{ListOrgMembersParamsDto, OrgMembershipDto, SwitchAuthContextDto, UserDto};

pub fn profile_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(profile_page_handler))
        .route("/profile-controls", get(profile_controls_handler))
        .route(
            "/switch-auth-context",
            get(switch_auth_context_handler).post(post_switch_auth_context_handler),
        )
        .route("/search-org", get(search_org_memberships_handler))
        .route("/select-org", get(select_org_handler))
        .route(
            "/change-password",
            get(change_current_password_handler).post(post_change_current_password_handler),
        )
        .with_state(state)
}

#[derive(Template)]
#[template(path = "pages/user/profile.html")]
struct ProfilePageTemplate {
    t: TemplateData,
    user: UserDto,
}

async fn profile_page_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);

    let actor = ctx.actor().expect("actor is required");
    t.title = format!("User - {}", &actor.user.name);

    let tpl = ProfilePageTemplate {
        t,
        user: actor.user.clone(),
    };

    Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)
}

#[derive(Template)]
#[template(path = "widgets/edit_profile_controls.html")]
struct ProfileControlsTemplate {}

async fn profile_controls_handler() -> Result<Response<Body>> {
    let tpl = ProfileControlsTemplate {};

    Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)
}

#[derive(Template)]
#[template(path = "widgets/change_user_password_form.html")]
struct ChangeUserPasswordTemplate {
    payload: ChangeCurrentPasswordFormData,
    error_message: Option<String>,
}

async fn change_current_password_handler(
    Extension(ctx): Extension<Ctx>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");

    let token = create_csrf_token_svc(actor.user.id.to_string().as_str(), &config.jwt_secret)?;

    let tpl = ChangeUserPasswordTemplate {
        payload: ChangeCurrentPasswordFormData {
            token,
            current_password: "".to_string(),
            new_password: "".to_string(),
            confirm_new_password: "".to_string(),
        },
        error_message: None,
    };

    Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)
}

async fn post_change_current_password_handler(
    Extension(ctx): Extension<Ctx>,
    State(state): State<AppState>,
    payload: Form<ChangeCurrentPasswordFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();
    let actor = ctx.actor().expect("actor is required");

    let token = create_csrf_token_svc(actor.user.id.to_string().as_str(), &config.jwt_secret)?;

    let mut tpl = ChangeUserPasswordTemplate {
        payload: ChangeCurrentPasswordFormData {
            token,
            current_password: payload.current_password.clone(),
            new_password: payload.new_password.clone(),
            confirm_new_password: payload.confirm_new_password.clone(),
        },
        error_message: None,
    };

    let data = ChangeCurrentPasswordFormData {
        token: payload.token.clone(),
        current_password: payload.current_password.clone(),
        new_password: payload.new_password.clone(),
        confirm_new_password: payload.confirm_new_password.clone(),
    };

    let result = change_user_current_password_svc(&state, &ctx, actor.user.id, data).await;

    match result {
        Ok(_) => {
            let tpl = ProfileControlsTemplate {};

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
#[template(path = "pages/user/switch_auth_context.html")]
struct SwitchAuthContextTemplate {
    t: TemplateData,
    payload: SwitchAuthContextFormData,
    query_params: String,
    error_message: Option<String>,
}

async fn switch_auth_context_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    State(state): State<AppState>,
    Query(query): Query<ListOrgMembersParamsDto>,
) -> Result<Response<Body>> {
    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);

    t.title = "Switch Organization".into();

    let mut next_url = "";

    if let Some(next) = query.next.as_deref() {
        next_url = next;
    }

    let tpl = SwitchAuthContextTemplate {
        t,
        payload: SwitchAuthContextFormData {
            token: "".to_string(),
            org_id: 0,
            org_name: "".to_string(),
            next: next_url.to_string(),
        },
        query_params: query.to_string(),
        error_message: None,
    };

    Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)
}

/// Full page submit handler for switching auth context
async fn post_switch_auth_context_handler(
    cookies: Cookies,
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    State(state): State<AppState>,
    Form(payload): Form<SwitchAuthContextFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let token = create_csrf_token_svc("org_membership", &config.jwt_secret)?;

    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);

    t.title = "Switch Organization".into();

    let mut tpl = SwitchAuthContextTemplate {
        t,
        payload: SwitchAuthContextFormData {
            token,
            org_id: payload.org_id,
            org_name: payload.org_name.clone(),
            next: payload.next.clone(),
        },
        query_params: "".to_string(),
        error_message: None,
    };

    let status: StatusCode;

    let result = switch_auth_context_svc(
        &state,
        ctx.token().expect("token is required"),
        SwitchAuthContextDto {
            org_id: payload.org_id,
        },
    )
    .await;

    match result {
        Ok(auth_response) => {
            let auth_cookie = Cookie::build((AUTH_TOKEN_COOKIE, auth_response.token))
                .http_only(true)
                .max_age(Duration::weeks(1))
                .secure(state.config.server.https)
                .path("/")
                .build();

            cookies.add(auth_cookie);

            let mut next_url = "/";

            if !payload.next.is_empty() && payload.next.starts_with('/') {
                next_url = &payload.next;
            }

            return Ok(Redirect::to(next_url).into_response());
        }
        Err(err) => {
            let error_info = ErrorInfo::from(&err);
            status = error_info.status_code;
            tpl.error_message = Some(error_info.message);
        }
    }

    tpl.payload.org_id = payload.org_id;
    tpl.payload.org_name = payload.org_name;

    // Will only arrive here on error
    Response::builder()
        .status(status)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)
}

#[derive(Template)]
#[template(path = "widgets/user/search_org.html")]
struct SearchOrgMembershipsTemplate {
    memberships: Vec<OrgMembershipDto>,
    pagination: Option<PaginationLinks>,
    error_message: Option<String>,
    next_url: String,
}
async fn search_org_memberships_handler(
    Extension(ctx): Extension<Ctx>,
    State(state): State<AppState>,
    Query(query): Query<ListOrgMembersParamsDto>,
) -> Result<Response<Body>> {
    let keyword = query.keyword.clone();
    let next = query.next.clone();

    let next_url = next.as_deref().unwrap_or("");

    let mut tpl = SearchOrgMembershipsTemplate {
        memberships: Vec::new(),
        pagination: None,
        error_message: None,
        next_url: next_url.to_string(),
    };

    match list_org_memberships_svc(&state, &ctx, query).await {
        Ok(memberships) => {
            let mut keyword_param: String = "".to_string();
            if let Some(keyword) = &keyword {
                keyword_param = format!("&keyword={}", encode(keyword));
            }

            if let Some(next) = &next {
                keyword_param = format!("{}&next={}", keyword_param, encode(next));
            }

            tpl.memberships = memberships.data;
            tpl.pagination = Some(PaginationLinks::new(
                &memberships.meta,
                "/users/search",
                "/users",
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
#[template(path = "widgets/user/select_org.html")]
struct SelectOrgTemplate {
    payload: SwitchAuthContextFormData,
    error_message: Option<String>,
}

async fn select_org_handler(
    State(state): State<AppState>,
    Query(params): Query<SwitchAuthContextParams>,
) -> Result<Response<Body>> {
    let token = create_csrf_token_svc("org_membership", &state.config.jwt_secret)?;

    let tpl = SelectOrgTemplate {
        payload: SwitchAuthContextFormData {
            token,
            org_id: params.org_id,
            org_name: params.org_name,
            next: params.next,
        },
        error_message: None,
    };

    Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)
}
