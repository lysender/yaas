use askama::Template;
use axum::debug_handler;
use axum::http::StatusCode;
use axum::{Extension, Form, body::Body, extract::State, response::Response};
use snafu::ResultExt;
use yaas::dto::OrgDto;
use yaas::dto::UserDto;
use yaas::role::Permission;

use crate::models::TokenFormData;
use crate::models::options::SelectOption;
use crate::services::users::delete_user_svc;
use crate::{
    Error, Result,
    ctx::Ctx,
    error::{ErrorInfo, ResponseBuilderSnafu, TemplateSnafu},
    models::{Pref, TemplateData},
    run::AppState,
    services::{
        token::create_csrf_token_svc,
        users::{
            NewUserFormData, ResetPasswordFormData, UserActiveFormData, UserRoleFormData,
            create_user_svc, list_users_svc, reset_user_password_svc, update_user_roles_svc,
            update_user_status_svc,
        },
    },
    web::{Action, Resource, enforce_policy},
};

#[derive(Template)]
#[template(path = "pages/users.html")]
struct UsersPageTemplate {
    t: TemplateData,
    org: OrgDto,
    users: Vec<UserDto>,
}

pub async fn users_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Read)?;

    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);
    t.title = String::from("Users");

    let token = ctx.token().expect("token is required");
    let users = list_users_svc(&state, token, org.id).await?;

    let tpl = UsersPageTemplate { t, org, users };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "pages/new_user.html")]
struct NewUserTemplate {
    t: TemplateData,
    org: OrgDto,
    action: String,
    payload: NewUserFormData,
    role_options: Vec<SelectOption>,
    error_message: Option<String>,
}

#[derive(Template)]
#[template(path = "widgets/new_user_form.html")]
struct NewUserFormTemplate {
    org: OrgDto,
    action: String,
    payload: NewUserFormData,
    role_options: Vec<SelectOption>,
    error_message: Option<String>,
}

fn create_role_options() -> Vec<SelectOption> {
    vec![
        SelectOption {
            value: "Admin".to_string(),
            label: "Admin".to_string(),
        },
        SelectOption {
            value: "Editor".to_string(),
            label: "Editor".to_string(),
        },
        SelectOption {
            value: "Viewer".to_string(),
            label: "Viewer".to_string(),
        },
    ]
}

pub async fn new_user_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Create)?;

    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);
    t.title = String::from("Create New User");

    let token = create_csrf_token_svc("new_user", &config.jwt_secret)?;
    let cid = org.id.clone();

    let tpl = NewUserTemplate {
        t,
        org,
        action: format!("/orgs/{}/users/new", cid),
        payload: NewUserFormData {
            username: "".to_string(),
            password: "".to_string(),
            confirm_password: "".to_string(),
            role: "".to_string(),
            token,
        },
        role_options: create_role_options(),
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

pub async fn post_new_user_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
    payload: Form<NewUserFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Create)?;

    let token = create_csrf_token_svc("new_user", &config.jwt_secret)?;
    let org_id = org.id;

    let mut tpl = NewUserFormTemplate {
        org,
        action: format!("/orgs/{}/users/new", org_id),
        payload: NewUserFormData {
            username: "".to_string(),
            password: "".to_string(),
            confirm_password: "".to_string(),
            role: "".to_string(),
            token,
        },
        role_options: create_role_options(),
        error_message: None,
    };

    let status: StatusCode;

    let user = NewUserFormData {
        username: payload.username.clone(),
        password: payload.password.clone(),
        confirm_password: payload.confirm_password.clone(),
        role: payload.role.clone(),
        token: payload.token.clone(),
    };

    let token = ctx.token().expect("token is required");
    let result = create_user_svc(&state, token, org_id, &user).await;

    match result {
        Ok(_) => {
            let next_url = format!("/orgs/{}/users", org_id);
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

    tpl.payload.username = payload.username.clone();
    tpl.payload.role = payload.role.clone();

    // Will only arrive here on error
    Ok(Response::builder()
        .status(status)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "pages/user.html")]
struct UserPageTemplate {
    t: TemplateData,
    org: OrgDto,
    user: UserDto,
    updated: bool,
    can_edit: bool,
    can_delete: bool,
}

pub async fn user_page_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Extension(org): Extension<OrgDto>,
    Extension(user): Extension<UserDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);

    t.title = format!("User - {}", &user.name);

    let tpl = UserPageTemplate {
        t,
        org,
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
#[template(path = "widgets/edit_user_controls.html")]
struct UserControlsTemplate {
    org: OrgDto,
    user: UserDto,
    updated: bool,
    can_edit: bool,
    can_delete: bool,
}

pub async fn user_controls_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    Extension(user): Extension<UserDto>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Update)?;

    let tpl = UserControlsTemplate {
        org,
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
#[template(path = "widgets/update_user_status_form.html")]
struct UpdateUserStatusTemplate {
    org: OrgDto,
    user: UserDto,
    payload: UserActiveFormData,
    error_message: Option<String>,
}

pub async fn update_user_status_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
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
        org,
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
    Extension(org): Extension<OrgDto>,
    Extension(user): Extension<UserDto>,
    State(state): State<AppState>,
    payload: Form<UserActiveFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Update)?;

    let token = create_csrf_token_svc(user.id.to_string().as_str(), &config.jwt_secret)?;
    let org_id = org.id;
    let user_id = user.id;

    let mut tpl = UpdateUserStatusTemplate {
        org: org.clone(),
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

    let token = ctx.token().expect("token is required");
    let result = update_user_status_svc(&state, token, org_id, user_id, &data).await;

    match result {
        Ok(updated_user) => {
            // Render back the controls but when updated roles and status
            let tpl = UserControlsTemplate {
                org,
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
#[template(path = "widgets/reset_user_password_form.html")]
struct ResetUserPasswordTemplate {
    org: OrgDto,
    user: UserDto,
    payload: ResetPasswordFormData,
    error_message: Option<String>,
}

pub async fn reset_user_password_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    Extension(user): Extension<UserDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Update)?;
    let token = create_csrf_token_svc(user.id.to_string().as_str(), &config.jwt_secret)?;

    let tpl = ResetUserPasswordTemplate {
        org,
        user,
        payload: ResetPasswordFormData {
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
pub async fn post_reset_password_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    Extension(user): Extension<UserDto>,
    State(state): State<AppState>,
    payload: Form<ResetPasswordFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Update)?;

    let token = create_csrf_token_svc(user.id.to_string().as_str(), &config.jwt_secret)?;
    let org_id = org.id;
    let user_id = user.id;

    let mut tpl = ResetUserPasswordTemplate {
        org: org.clone(),
        user: user.clone(),
        payload: ResetPasswordFormData {
            token,
            password: payload.password.clone(),
            confirm_password: payload.confirm_password.clone(),
        },
        error_message: None,
    };

    let data = ResetPasswordFormData {
        token: payload.token.clone(),
        password: payload.password.clone(),
        confirm_password: payload.confirm_password.clone(),
    };

    let token = ctx.token().expect("token is required");
    let result = reset_user_password_svc(&state, token, org_id, user_id, &data).await;

    match result {
        Ok(_) => {
            let tpl = UserControlsTemplate {
                org,
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
#[template(path = "widgets/delete_user_form.html")]
struct DeleteUserFormTemplate {
    org: OrgDto,
    user: UserDto,
    payload: TokenFormData,
    error_message: Option<String>,
}

pub async fn delete_user_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    Extension(user): Extension<UserDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Delete)?;

    let token = create_csrf_token_svc(user.id.to_string().as_str(), &config.jwt_secret)?;

    let tpl = DeleteUserFormTemplate {
        org,
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
    Extension(org): Extension<OrgDto>,
    Extension(user): Extension<UserDto>,
    State(state): State<AppState>,
    payload: Form<TokenFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Delete)?;

    let token = create_csrf_token_svc(user.id.to_string().as_str(), &config.jwt_secret)?;

    let mut tpl = DeleteUserFormTemplate {
        org: org.clone(),
        user: user.clone(),
        payload: TokenFormData { token },
        error_message: None,
    };

    let token = ctx.token().expect("token is required");
    let result = delete_user_svc(&state, token, org.id, user.id, &payload.token).await;

    match result {
        Ok(_) => {
            // Render same form but trigger a redirect to home
            let cid = org.id.clone();
            let tpl = DeleteUserFormTemplate {
                org,
                user,
                payload: TokenFormData {
                    token: "".to_string(),
                },
                error_message: None,
            };
            return Ok(Response::builder()
                .status(200)
                .header("HX-Redirect", format!("/orgs/{}/users", &cid))
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
