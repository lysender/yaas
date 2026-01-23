use askama::Template;
use axum::{
    body::Body,
    extract::{Form, Query, State},
    http::Response,
    response::{IntoResponse, Redirect},
};
use snafu::ResultExt;
use std::collections::HashMap;
use tower_cookies::{Cookie, Cookies, cookie::time::Duration};
use validator::Validate;

use crate::{
    Error, Result,
    error::{ResponseBuilderSnafu, TemplateSnafu},
    models::{LoginFormPayload, TemplateData},
    services::auth::authenticate,
};
use crate::{error::ErrorInfo, models::Pref, run::AppState};
use yaas::{
    dto::{Actor, CredentialsDto},
    validators::flatten_errors,
};

use super::AUTH_TOKEN_COOKIE;

#[derive(Template)]
#[template(path = "pages/login.html")]
struct LoginTemplate {
    t: TemplateData,
    error_message: Option<String>,
}

pub async fn login_handler(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Response<Body>> {
    // Errors are handled via redirect with query params
    let pref = Pref::new();
    let actor = Actor::default();
    let mut t = TemplateData::new(&state, actor, &pref);
    t.title = String::from("Login");

    let mut error_message = None;
    if let Some(err) = query.get("error") {
        error_message = Some(err.to_string());
    }

    let tpl = LoginTemplate { t, error_message };

    Response::builder()
        .status(200)
        .header("Surrogate-Control", "no-store")
        .header(
            "Cache-Control",
            "no-store, no-cache, must-revalidate, proxy-revalidate",
        )
        .header("Pragma", "no-cache")
        .header("Expires", 0)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)
}

pub async fn post_login_handler(
    cookies: Cookies,
    State(state): State<AppState>,
    Form(login_payload): Form<LoginFormPayload>,
) -> impl IntoResponse {
    // Validate data
    if let Err(err) = login_payload.validate() {
        let msg = flatten_errors(&err);
        return handle_error(Error::Validation { msg });
    }

    // Validate login information
    let auth_payload = CredentialsDto {
        email: login_payload.username,
        password: login_payload.password,
    };
    let login_result = authenticate(&state, auth_payload).await;
    let auth = match login_result {
        Ok(val) => val,
        Err(err) => {
            return handle_error(err);
        }
    };

    let auth_cookie = Cookie::build((AUTH_TOKEN_COOKIE, auth.token.clone()))
        .http_only(true)
        .max_age(Duration::weeks(1))
        .secure(state.config.server.https)
        .path("/")
        .build();

    cookies.add(auth_cookie);

    let redirect_url = if auth.org_count > 1 {
        "/profile/switch-auth-context"
    } else {
        "/"
    };

    Redirect::to(redirect_url).into_response()
}

fn handle_error(error: Error) -> Response<Body> {
    let error_info = ErrorInfo::from(&error);

    let url = format!("/login?error={}", error_info.message);
    Redirect::to(url.as_str()).into_response()
}
