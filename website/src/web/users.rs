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
use crate::services::users::{ChangePasswordFormData, change_user_password_svc, delete_user_svc};
use crate::{
    Error, Result,
    ctx::Ctx,
    error::{ErrorInfo, ResponseBuilderSnafu, TemplateSnafu},
    models::{Pref, TemplateData},
    run::AppState,
    services::{
        token::create_csrf_token_svc,
        users::{
            NewUserFormData, UserActiveFormData, create_user_svc, list_users_svc,
            update_user_status_svc,
        },
    },
    web::{Action, Resource, enforce_policy},
};
use yaas::dto::ListUsersParamsDto;
use yaas::dto::UserDto;
use yaas::role::Permission;

#[derive(Template)]
#[template(path = "pages/users/index.html")]
struct UsersPageTemplate {
    t: TemplateData,
    query_params: String,
}

pub async fn users_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    State(state): State<AppState>,
    Query(query): Query<ListUsersParamsDto>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Read)?;

    let errors = query.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);
    t.title = String::from("Users");

    let tpl = UsersPageTemplate {
        t,
        query_params: query.to_string(),
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/users/search.html")]
struct SearchUsersTemplate {
    users: Vec<UserDto>,
    pagination: Option<PaginationLinks>,
    error_message: Option<String>,
}
pub async fn search_users_handler(
    Extension(ctx): Extension<Ctx>,
    State(state): State<AppState>,
    Query(query): Query<ListUsersParamsDto>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Read)?;

    let mut tpl = SearchUsersTemplate {
        users: Vec::new(),
        pagination: None,
        error_message: None,
    };

    let keyword = query.keyword.clone();

    match list_users_svc(&state, &ctx, query).await {
        Ok(users) => {
            let mut keyword_param: String = "".to_string();
            if let Some(keyword) = &keyword {
                keyword_param = format!("&keyword={}", encode(keyword).to_string());
            }
            tpl.users = users.data;
            tpl.pagination = Some(PaginationLinks::new(&users.meta, "", &keyword_param));

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
#[template(path = "pages/users/new.html")]
struct NewUserTemplate {
    t: TemplateData,
    action: String,
    payload: NewUserFormData,
    error_message: Option<String>,
}

#[derive(Template)]
#[template(path = "widgets/users/new_form.html")]
struct NewUserFormTemplate {
    action: String,
    payload: NewUserFormData,
    error_message: Option<String>,
}

pub async fn new_user_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Create)?;

    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);
    t.title = String::from("Create New User");

    let token = create_csrf_token_svc("new_user", &config.jwt_secret)?;

    let tpl = NewUserTemplate {
        t,
        action: "/users/new".to_string(),
        payload: NewUserFormData {
            name: "".to_string(),
            email: "".to_string(),
            password: "".to_string(),
            confirm_password: "".to_string(),
            token,
        },
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

pub async fn post_new_user_handler(
    Extension(ctx): Extension<Ctx>,
    State(state): State<AppState>,
    Form(payload): Form<NewUserFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Create)?;

    let token = create_csrf_token_svc("new_user", &config.jwt_secret)?;

    let mut tpl = NewUserFormTemplate {
        action: "/users/new".to_string(),
        payload: NewUserFormData {
            name: "".to_string(),
            email: "".to_string(),
            password: "".to_string(),
            confirm_password: "".to_string(),
            token,
        },
        error_message: None,
    };

    let status: StatusCode;

    let user = NewUserFormData {
        name: payload.name.clone(),
        email: payload.email.clone(),
        password: payload.password.clone(),
        confirm_password: payload.confirm_password.clone(),
        token: payload.token.clone(),
    };

    let result = create_user_svc(&state, &ctx, user).await;

    match result {
        Ok(_) => {
            let next_url = "/users".to_string();
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
    tpl.payload.email = payload.email.clone();

    // Will only arrive here on error
    Ok(Response::builder()
        .status(status)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "pages/users/view.html")]
struct UserPageTemplate {
    t: TemplateData,
    user: UserDto,
    updated: bool,
    can_edit: bool,
    can_delete: bool,
}

pub async fn user_page_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Extension(user): Extension<UserDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);

    t.title = format!("User - {}", &user.email);

    let tpl = UserPageTemplate {
        t,
        user,
        updated: false,
        can_edit: ctx.actor.has_permissions(&vec![Permission::UsersEdit]),
        can_delete: ctx.actor.has_permissions(&vec![Permission::UsersDelete]),
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/users/edit_controls.html")]
struct UserControlsTemplate {
    user: UserDto,
    updated: bool,
    can_edit: bool,
    can_delete: bool,
}

pub async fn user_controls_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(user): Extension<UserDto>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Update)?;

    let tpl = UserControlsTemplate {
        user,
        updated: false,
        can_edit: ctx.actor.has_permissions(&vec![Permission::UsersEdit]),
        can_delete: ctx.actor.has_permissions(&vec![Permission::UsersDelete]),
    };

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/users/update_status_form.html")]
struct UpdateUserStatusTemplate {
    user: UserDto,
    payload: UserActiveFormData,
    error_message: Option<String>,
}

pub async fn update_user_status_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(user): Extension<UserDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Update)?;
    let token = create_csrf_token_svc(user.id.to_string().as_str(), &config.jwt_secret)?;

    let mut status_opt = None;
    if &user.status == "active" {
        status_opt = Some("1".to_string());
    }

    let tpl = UpdateUserStatusTemplate {
        user,
        payload: UserActiveFormData {
            token,
            active: status_opt,
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
pub async fn post_update_user_status_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(user): Extension<UserDto>,
    State(state): State<AppState>,
    payload: Form<UserActiveFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Update)?;

    let token = create_csrf_token_svc(&user.id.to_string(), &config.jwt_secret)?;
    let user_id = user.id;

    let mut tpl = UpdateUserStatusTemplate {
        user,
        payload: UserActiveFormData {
            token,
            active: payload.active.clone(),
        },
        error_message: None,
    };

    let data = UserActiveFormData {
        active: payload.active.clone(),
        token: payload.token.clone(),
    };

    let result = update_user_status_svc(&state, &ctx, user_id, data).await;

    match result {
        Ok(updated_user) => {
            // Render back the controls but when updated roles and status
            let tpl = UserControlsTemplate {
                user: updated_user,
                updated: true,
                can_edit: ctx.actor.has_permissions(&vec![Permission::UsersEdit]),
                can_delete: ctx.actor.has_permissions(&vec![Permission::UsersDelete]),
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
#[template(path = "widgets/users/change_password_form.html")]
struct ChangePasswordTemplate {
    user: UserDto,
    payload: ChangePasswordFormData,
    error_message: Option<String>,
}

pub async fn change_password_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(user): Extension<UserDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Update)?;
    let token = create_csrf_token_svc(&user.id.to_string(), &config.jwt_secret)?;

    let tpl = ChangePasswordTemplate {
        user,
        payload: ChangePasswordFormData {
            token,
            password: "".to_string(),
            confirm_password: "".to_string(),
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
pub async fn post_change_password_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(user): Extension<UserDto>,
    State(state): State<AppState>,
    payload: Form<ChangePasswordFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Update)?;

    let token = create_csrf_token_svc(&user.id.to_string(), &config.jwt_secret)?;
    let user_id = user.id;

    let mut tpl = ChangePasswordTemplate {
        user: user.clone(),
        payload: ChangePasswordFormData {
            token,
            password: payload.password.clone(),
            confirm_password: payload.confirm_password.clone(),
        },
        error_message: None,
    };

    let data = ChangePasswordFormData {
        token: payload.token.clone(),
        password: payload.password.clone(),
        confirm_password: payload.confirm_password.clone(),
    };

    let result = change_user_password_svc(&state, &ctx, user_id, data).await;

    match result {
        Ok(_) => {
            let tpl = UserControlsTemplate {
                user,
                updated: false,
                can_edit: ctx.actor.has_permissions(&vec![Permission::UsersEdit]),
                can_delete: ctx.actor.has_permissions(&vec![Permission::UsersDelete]),
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
#[template(path = "widgets/users/delete_form.html")]
struct DeleteUserFormTemplate {
    user: UserDto,
    payload: TokenFormData,
    error_message: Option<String>,
}

pub async fn delete_user_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(user): Extension<UserDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Delete)?;

    let token = create_csrf_token_svc(&user.id.to_string(), &config.jwt_secret)?;

    let tpl = DeleteUserFormTemplate {
        user,
        payload: TokenFormData { token },
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

pub async fn post_delete_user_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(user): Extension<UserDto>,
    State(state): State<AppState>,
    payload: Form<TokenFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Delete)?;

    let token = create_csrf_token_svc(&user.id.to_string(), &config.jwt_secret)?;

    let mut tpl = DeleteUserFormTemplate {
        user: user.clone(),
        payload: TokenFormData { token },
        error_message: None,
    };

    let result = delete_user_svc(&state, &ctx, user.id, &payload.token).await;

    match result {
        Ok(_) => {
            // Render same form but trigger a redirect to home
            let tpl = DeleteUserFormTemplate {
                user,
                payload: TokenFormData {
                    token: "".to_string(),
                },
                error_message: None,
            };
            return Ok(Response::builder()
                .status(200)
                .header("HX-Redirect", "/users".to_string())
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
