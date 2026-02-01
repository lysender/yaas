use askama::Template;
use axum::{
    Extension, Json,
    body::Body,
    extract::{Query, State, rejection::JsonRejection},
    http::Response,
    response::{IntoResponse, Redirect},
};
use snafu::ResultExt;
use validator::Validate;

use crate::{
    Result,
    ctx::Ctx,
    error::{JsonSerializeSnafu, ResponseBuilderSnafu, TemplateSnafu},
    models::{Pref, TemplateData},
    run::AppState,
    services::{create_authorization_code, exchange_code_for_access_token},
};
use yaas::{
    dto::{Actor, ErrorMessageDto, OauthAuthorizeDto, OauthTokenRequestDto, OauthTokenResponseDto},
    validators::flatten_errors,
};

#[derive(Template)]
#[template(path = "pages/oauth_error.html")]
struct OauthErrorTemplate {
    t: TemplateData,
    error_message: String,
}

pub async fn oauth_authorize_handler(
    State(state): State<AppState>,
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Query(query): Query<OauthAuthorizeDto>,
) -> Result<Response<Body>> {
    // Validate query parameters
    if let Err(err) = query.validate() {
        let msg = flatten_errors(&err);
        return render_error(&state, ctx.actor, &pref, msg);
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
            let error_info = crate::error::ErrorInfo::from(&err);

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
                render_error(&state, ctx.actor, &pref, error_info.message)
            }
        }
    }
}

fn render_error(
    state: &AppState,
    actor: Actor,
    pref: &Pref,
    error_message: String,
) -> Result<Response<Body>> {
    let mut t = TemplateData::new(state, actor, pref);
    t.title = String::from("OAuth Authorization Error");

    let tpl = OauthErrorTemplate { t, error_message };

    Response::builder()
        .status(400)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)
}

pub async fn oauth_token_handler(
    State(state): State<AppState>,
    payload: core::result::Result<Json<OauthTokenRequestDto>, JsonRejection>,
) -> Result<Response<Body>> {
    match payload {
        Ok(data) => {
            // Validate query parameters
            if let Err(err) = data.validate() {
                let msg = flatten_errors(&err);
                return render_json_error(400, "validation_error", msg);
            }

            let result = exchange_code_for_access_token(&state, &data).await;

            match result {
                Ok(token_response) => {
                    let json_response = OauthTokenResponseDto {
                        access_token: token_response.access_token,
                        scope: token_response.scope,
                        token_type: token_response.token_type,
                    };

                    let json_body =
                        serde_json::to_string(&json_response).context(JsonSerializeSnafu)?;

                    Response::builder()
                        .status(200)
                        .header("Content-Type", "application/json")
                        .body(Body::from(json_body))
                        .context(ResponseBuilderSnafu)
                }
                Err(err) => {
                    let error_info = crate::error::ErrorInfo::from(&err);
                    render_json_error(
                        error_info.status_code.as_u16(),
                        "oauth_error",
                        error_info.message,
                    )
                }
            }
        }
        Err(json_err) => {
            let msg = format!("Invalid JSON payload: {}", json_err);
            return render_json_error(400, "invalid_request", msg);
        }
    }
}

fn render_json_error(status: u16, error: &str, message: String) -> Result<Response<Body>> {
    let error_response = ErrorMessageDto {
        status_code: status,
        error: error.to_string(),
        message,
    };

    let json_body = serde_json::to_string(&error_response).context(JsonSerializeSnafu)?;

    Response::builder()
        .status(status)
        .header("Content-Type", "application/json")
        .body(Body::from(json_body))
        .context(ResponseBuilderSnafu)
}
