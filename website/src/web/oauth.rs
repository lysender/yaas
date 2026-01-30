use askama::Template;
use axum::{
    Extension,
    body::Body,
    extract::{Query, State},
    http::Response,
    response::{IntoResponse, Redirect},
};
use serde::Deserialize;
use snafu::ResultExt;
use tracing::info;
use validator::Validate;

use crate::{
    Result,
    ctx::Ctx,
    error::{ResponseBuilderSnafu, TemplateSnafu},
    models::{Pref, TemplateData},
    run::AppState,
    services::create_authorization_code,
};
use yaas::{dto::Actor, validators::flatten_errors};

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct OauthAuthorizeQuery {
    #[validate(length(equal = 36))]
    pub client_id: String,

    #[validate(url)]
    #[validate(length(min = 1, max = 250))]
    pub redirect_uri: String,

    #[validate(length(min = 1, max = 250))]
    pub scope: String,

    #[validate(length(min = 1, max = 250))]
    pub state: String,
}

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
    Query(query): Query<OauthAuthorizeQuery>,
) -> Result<Response<Body>> {
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

    info!("validating query params");

    // Validate query parameters
    if let Err(err) = query.validate() {
        let msg = flatten_errors(&err);
        return render_error(&state, ctx.actor, &pref, msg);
    }

    info!("trying to call api");
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
