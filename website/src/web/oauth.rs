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
use validator::Validate;

use crate::{
    Error, Result,
    ctx::Ctx,
    error::{ErrorInfo, JsonRejectionSnafu, JsonSerializeSnafu, ResponseBuilderSnafu},
    models::Pref,
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

        return Ok(handle_error(&state, ctx.actor, &pref, error_info, true));
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
            // Success: redirect to redirect_uri with code and state
            let redirect_url = format!(
                "{}?code={}&state={}",
                query.redirect_uri,
                urlencoding::encode(&auth_code.code),
                urlencoding::encode(&auth_code.state)
            );
            Ok(Redirect::to(&redirect_url).into_response())
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
                Ok(Redirect::to(&redirect_url).into_response())
            } else {
                Ok(handle_error(&state, ctx.actor, &pref, error_info, true))
            }
        }
    }
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

    if let Some(auth_header) = headers.get("Authorization") {
        if let Ok(auth_str) = auth_header.to_str() {
            if auth_str.starts_with("Bearer ") {
                token = Some(auth_str[7..].to_string());
            }
        }
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
