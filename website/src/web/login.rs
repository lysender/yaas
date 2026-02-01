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
    next: Option<String>,
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

    let next = query.get("next").cloned();

    let tpl = LoginTemplate {
        t,
        error_message,
        next,
    };

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
        return handle_error(Error::Validation { msg }, login_payload.next.as_deref());
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
            return handle_error(err, login_payload.next.as_deref());
        }
    };

    let auth_cookie = Cookie::build((AUTH_TOKEN_COOKIE, auth.token.clone()))
        .http_only(true)
        .max_age(Duration::weeks(1))
        .secure(state.config.server.https)
        .same_site(tower_cookies::cookie::SameSite::Strict)
        .path("/")
        .build();

    cookies.add(auth_cookie);

    let mut redirect_url = "/".to_string();

    if auth.org_count > 1 {
        // Let the user choose which org to use
        redirect_url = "/profile/switch-auth-context".to_string();

        if let Some(next) = login_payload.next {
            // Add some query parameter to the redirect url so it knows where to redirect further
            redirect_url = format!("{}?next={}", redirect_url, urlencoding::encode(&next));
        }
    } else {
        if let Some(next) = login_payload.next {
            redirect_url = next;
        }
    }

    Redirect::to(&redirect_url).into_response()
}

fn handle_error(error: Error, next: Option<&str>) -> Response<Body> {
    let error_info = ErrorInfo::from(&error);

    let mut url = format!("/login?error={}", urlencoding::encode(&error_info.message));
    if let Some(next_url) = next {
        url.push_str(&format!("&next={}", urlencoding::encode(next_url)));
    }

    Redirect::to(&url).into_response()
}
