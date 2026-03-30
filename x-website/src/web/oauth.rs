use askama::Template;
use axum::{
    Extension, Json, Router,
    body::Body,
    extract::{Query, State, rejection::JsonRejection},
    http::{HeaderMap, StatusCode},
    middleware,
    response::{IntoResponse, Redirect, Response},
    routing::{get, post},
};
use snafu::ResultExt;
use tracing::error;
use url::Url;
use validator::Validate;

use crate::{
    Error, Result,
    ctx::Ctx,
    error::{
        ErrorInfo, JsonRejectionSnafu, JsonSerializeSnafu, ResponseBuilderSnafu, TemplateSnafu,
    },
    models::{CspNonce, Pref, TemplateData},
    run::AppState,
    services::{create_authorization_code, exchange_code_for_access_token, oauth_profile},
    web::handle_error,
};
use yaas::{
    dto::{ErrorMessageDto, OauthAuthorizeDto, OauthTokenRequestDto},
    validators::flatten_errors,
};

/// OAuth API Routes are handled differently from the rest of the web routes.
/// Responses and errors are in JSON format, and authentication is validated within the handlers.
pub fn oauth_api_routes(state: AppState) -> Router {
    Router::new()
        .route("/oauth/token", post(oauth_token_handler))
        .route("/oauth/profile", get(oauth_profile_handler))
        .layer(middleware::map_response_with_state(
            state.clone(),
            api_response_mapper,
        ))
        .with_state(state)
}

/// Web handler for OAuth2 Authorization Endpoint
pub async fn oauth_authorize_handler(
    State(state): State<AppState>,
    Extension(csp_nonce): Extension<CspNonce>,
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Query(query): Query<OauthAuthorizeDto>,
) -> Result<Response<Body>> {
    // Validate query parameters
    if let Err(err) = query.validate() {
        let msg = flatten_errors(&err);
        let error_info = ErrorInfo {
            status_code: StatusCode::BAD_REQUEST,
            title: "Invalid Request".to_string(),
            message: msg,
        };

        return Ok(handle_error(
            &state,
            ctx.actor,
            &pref,
            csp_nonce.nonce,
            error_info,
            true,
        ));
    }

    // Check if user is logged in
    if !ctx.actor.has_auth_scope() {
        let current_path = format!(
            "/oauth/authorize?client_id={}&redirect_uri={}&scope={}&state={}",
            urlencoding::encode(&query.client_id),
            urlencoding::encode(&query.redirect_uri),
            urlencoding::encode(&query.scope),
            urlencoding::encode(&query.state),
        );
        let login_url = format!("/login?next={}", urlencoding::encode(&current_path));
        return Ok(Redirect::to(&login_url).into_response());
    }

    let result = create_authorization_code(&state, &ctx, &query).await;

    match result {
        Ok(auth_code) => {
            // Success: redirect to resume page before leaving this origin
            let redirect_url = format!(
                "{}?code={}&state={}",
                query.redirect_uri,
                urlencoding::encode(&auth_code.code),
                urlencoding::encode(&auth_code.state)
            );
            let resume_url = format!(
                "/oauth/authorize/resume?next={}",
                urlencoding::encode(&redirect_url)
            );
            Ok(Redirect::to(&resume_url).into_response())
        }
        Err(err) => {
            // Error: redirect to redirect_uri with error details if possible
            let error_info = ErrorInfo::from(&err);

            // Only redirect to redirect_uri if it's a valid URL
            // Otherwise, render error page
            if query.redirect_uri.starts_with("http://")
                || query.redirect_uri.starts_with("https://")
            {
                let redirect_url = format!(
                    "{}?error=access_denied&error_description={}&state={}",
                    query.redirect_uri,
                    urlencoding::encode(&error_info.message),
                    urlencoding::encode(&query.state)
                );
                let resume_url = format!(
                    "/oauth/authorize/resume?next={}",
                    urlencoding::encode(&redirect_url)
                );
                Ok(Redirect::to(&resume_url).into_response())
            } else {
                Ok(handle_error(
                    &state,
                    ctx.actor,
                    &pref,
                    csp_nonce.nonce,
                    error_info,
                    true,
                ))
            }
        }
    }
}

#[derive(Template)]
#[template(path = "pages/oauth_authorize_resume.html")]
struct OauthAuthorizeResumeTemplate {
    t: TemplateData,
    next: String,
    app_name: String,
    refresh_content: String,
}

pub async fn oauth_authorize_resume_handler(
    State(state): State<AppState>,
    Extension(csp_nonce): Extension<CspNonce>,
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Query(query): Query<std::collections::HashMap<String, String>>,
) -> Result<Response<Body>> {
    let next = query.get("next").cloned().unwrap_or_default();

    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref, csp_nonce.nonce);
    t.title = String::from("Redirecting");

    let tpl = OauthAuthorizeResumeTemplate {
        t,
        app_name: resolve_app_name_from_next(&next),
        refresh_content: format!("1;url={}", next),
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

fn resolve_app_name_from_next(next: &str) -> String {
    let Ok(url) = Url::parse(next) else {
        return "your app".to_string();
    };

    let Some(host) = url.host_str() else {
        return "your app".to_string();
    };

    host.to_string()
}

/// API handler for OAuth2 Token Endpoint
/// Exchange authorization code for access token
pub async fn oauth_token_handler(
    State(state): State<AppState>,
    payload: core::result::Result<Json<OauthTokenRequestDto>, JsonRejection>,
) -> Result<Response<Body>> {
    let data = payload.context(JsonRejectionSnafu)?;

    // Validate query parameters
    if let Err(err) = data.validate() {
        let msg = flatten_errors(&err);
        return Err(Error::Validation { msg });
    }

    let oauth_token = exchange_code_for_access_token(&state, &data).await?;

    let json_body = serde_json::to_string(&oauth_token).context(JsonSerializeSnafu)?;

    Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(json_body))
        .context(ResponseBuilderSnafu)
}

/// API handler for OAuth2 User Profile Endpoint
/// Fetch user profile using access token
pub async fn oauth_profile_handler(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Response<Body>> {
    // Manually validate auth token
    let mut token: Option<String> = None;

    if let Some(auth_header) = headers.get("Authorization")
        && let Ok(auth_str) = auth_header.to_str()
        && auth_str.starts_with("Bearer ")
    {
        token = Some(auth_str[7..].to_string());
    }

    let Some(token) = token else {
        return Err(Error::LoginRequired);
    };

    let user = oauth_profile(&state, &token).await?;
    let json_body = serde_json::to_string(&user).context(JsonSerializeSnafu)?;

    Response::builder()
        .status(200)
        .header("Content-Type", "application/json")
        .body(Body::from(json_body))
        .context(ResponseBuilderSnafu)
}

async fn api_response_mapper(res: Response) -> Response {
    let error = res.extensions().get::<ErrorInfo>();
    if let Some(e) = error {
        if e.status_code.is_server_error() {
            // Build the error response
            error!("{}", e.message);
        }

        let error_message = ErrorMessageDto {
            status_code: e.status_code.as_u16(),
            message: e.message.clone(),
            error: e.status_code.canonical_reason().unwrap().to_string(),
            error_code: None,
        };

        let json_body = serde_json::to_string(&error_message).unwrap_or("{}".to_string());

        return Response::builder()
            .status(e.status_code)
            .header("Content-Type", "application/json")
            .body(Body::from(json_body))
            .unwrap();
    }
    res
}

#[cfg(test)]
mod tests {
    use super::resolve_app_name_from_next;

    #[test]
    fn resolve_app_name_uses_host() {
        let app_name =
            resolve_app_name_from_next("https://photos.example.com/oauth/callback?code=c1");
        assert_eq!(app_name, "photos.example.com");
    }

    #[test]
    fn resolve_app_name_falls_back_for_invalid_url() {
        let app_name = resolve_app_name_from_next("not-a-url");
        assert_eq!(app_name, "your app");
    }
}
