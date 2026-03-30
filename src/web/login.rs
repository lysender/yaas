use askama::Template;
use axum::{
    Extension,
    body::Body,
    extract::{Form, Query, State},
    http::Response,
    response::{IntoResponse, Redirect},
};
use snafu::ResultExt;
use std::collections::HashMap;
use tower_cookies::{Cookie, Cookies, cookie::time::Duration};
use url::{Url, form_urlencoded};
use validator::Validate;

use crate::dto::{Actor, CredentialsDto, OauthClientLookupDto};
use crate::{
    Error, Result,
    error::{ResponseBuilderSnafu, TemplateSnafu},
    models::{CspNonce, LoginFormPayload, TemplateData},
    services::{auth::authenticate, captcha::validate_catpcha, lookup_oauth_client_app},
};
use crate::{error::ErrorInfo, models::Pref, run::AppState};

use super::AUTH_TOKEN_COOKIE;

#[derive(Template)]
#[template(path = "pages/login.html")]
struct LoginTemplate {
    t: TemplateData,
    login_title: String,
    captcha_key: String,
    captcha_enabled: bool,
    success_message: Option<String>,
    error_message: Option<String>,
    next: Option<String>,
}

pub async fn login_handler(
    Extension(csp_nonce): Extension<CspNonce>,
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Response<Body>> {
    // Errors are handled via redirect with query params
    let pref = Pref::new();
    let actor = Actor::default();
    let mut t = TemplateData::new(&state, actor, &pref, csp_nonce.nonce);
    t.title = String::from("Login");

    let config = state.config.clone();
    let captcha_enabled = config.captcha_enabled();
    if captcha_enabled {
        t.async_scripts = vec!["https://www.google.com/recaptcha/enterprise.js".to_string()];
    }
    let captcha_key = config.captcha_site_key.clone().unwrap_or_default();

    let success_message = query.get("success").cloned();
    let error_message = query.get("error").cloned();
    let next = query.get("next").cloned();
    let login_title = resolve_login_title(&state, next.as_deref()).await;

    let tpl = LoginTemplate {
        t,
        login_title,
        captcha_key,
        captcha_enabled,
        success_message,
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
    let captcha_enabled = state.config.captcha_enabled();

    // Validate data
    if let Err(err) = login_payload.validate() {
        let errors: Vec<String> = err
            .field_errors()
            .keys()
            .map(|k| match k.as_ref() {
                "g-recaptcha-response" => "captcha".to_string(),
                other => other.to_string(),
            })
            .collect();
        let mut error_message = "Complete the form.".to_string();
        if captcha_enabled && errors.contains(&"captcha".to_string()) {
            error_message = "Click the I'm not a robot checkbox.".to_string();
        }
        return handle_error(
            Error::Validation { msg: error_message },
            login_payload.next.as_deref(),
        );
    }

    // Validate captcha
    if captcha_enabled {
        let captcha_response = match login_payload.g_recaptcha_response.as_deref() {
            Some(value) if !value.trim().is_empty() => value,
            _ => {
                return handle_error(
                    Error::Validation {
                        msg: "Click the I'm not a robot checkbox.".into(),
                    },
                    login_payload.next.as_deref(),
                );
            }
        };

        if let Err(captcha_err) = validate_catpcha(&state, captcha_response).await {
            return handle_error(captcha_err, login_payload.next.as_deref());
        }
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
    } else if let Some(next) = login_payload.next {
        redirect_url = next;
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

async fn resolve_login_title(state: &AppState, next: Option<&str>) -> String {
    let Some(payload) = oauth_client_lookup_payload(next) else {
        return "Login to YAAS".to_string();
    };

    match lookup_oauth_client_app(state, &payload).await {
        Ok(app) => format!("Login to {}", app.name),
        Err(_) => "Login to YAAS".to_string(),
    }
}

fn oauth_client_lookup_payload(next: Option<&str>) -> Option<OauthClientLookupDto> {
    let next = next?.trim();

    let query = if let Some(query) = next.strip_prefix("/oauth/authorize?") {
        query.to_string()
    } else {
        let url = Url::parse(next).ok()?;
        if url.path() != "/oauth/authorize" {
            return None;
        }
        url.query()?.to_string()
    };

    let mut client_id: Option<String> = None;
    let mut redirect_uri: Option<String> = None;

    for (key, value) in form_urlencoded::parse(query.as_bytes()) {
        if key == "client_id" {
            client_id = Some(value.into_owned());
        } else if key == "redirect_uri" {
            redirect_uri = Some(value.into_owned());
        }
    }

    let payload = OauthClientLookupDto {
        client_id: client_id?,
        redirect_uri: redirect_uri?,
    };

    if payload.validate().is_err() {
        return None;
    }

    Some(payload)
}

#[cfg(test)]
mod tests {
    use super::oauth_client_lookup_payload;

    #[test]
    fn extracts_payload_from_relative_oauth_next() {
        let next = Some(
            "/oauth/authorize?client_id=123e4567-e89b-12d3-a456-426614174000&redirect_uri=https%3A%2F%2Fexample.com%2Fcallback&scope=oauth&state=s1",
        );

        let payload = oauth_client_lookup_payload(next).expect("Expected payload");

        assert_eq!(payload.client_id, "123e4567-e89b-12d3-a456-426614174000");
        assert_eq!(payload.redirect_uri, "https://example.com/callback");
    }

    #[test]
    fn returns_none_for_invalid_client_id() {
        let next = Some(
            "/oauth/authorize?client_id=bad-client&redirect_uri=https%3A%2F%2Fexample.com%2Fcallback&scope=oauth&state=s1",
        );

        assert!(oauth_client_lookup_payload(next).is_none());
    }

    #[test]
    fn returns_none_for_non_oauth_next() {
        let next = Some("/profile");
        assert!(oauth_client_lookup_payload(next).is_none());
    }
}
