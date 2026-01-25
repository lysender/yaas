use axum::{
    Extension, Router,
    body::{Body, Bytes},
    extract::State,
    response::Response,
    routing::post,
};
use prost::Message;
use snafu::{OptionExt, ResultExt, ensure};
use validator::Validate;

use crate::{
    Error, Result,
    error::{InvalidClientSnafu, ValidationSnafu},
    services::oauth_code::create_oauth_code_svc,
    state::AppState,
    web::build_response,
};
use yaas::{
    buffed::dto::{OauthAuthorizationCodeBuf, OauthAuthorizeBuf},
    dto::{Actor, NewOauthCodeDto, OauthAuthorizeDto},
    utils::{generate_id, validate_redirect_uri},
    validators::flatten_errors,
};

pub fn oauth_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/authorize", post(oauth_authorize_handler))
        .with_state(state)
}

/// POST /oauth/authorize
/// Initiates OAuth2 authorization code flow
/// Requires authenticated user (JWT Bearer token)
async fn oauth_authorize_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    body: Bytes,
) -> Result<Response<Body>> {
    let Ok(payload) = OauthAuthorizeBuf::decode(body) else {
        return Err(Error::BadProtobuf);
    };

    // Convert to dto for validation
    let data: OauthAuthorizeDto = payload.into();
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let actor_dto = actor.actor.context(InvalidClientSnafu)?;

    // Validate client_id from the apps table
    let app = state
        .db
        .apps
        .find_by_client_id(&data.client_id)
        .await
        .context(crate::error::DbSnafu)?;

    let app = app.context(InvalidClientSnafu)?;

    // Ensure redirect_uri is valid and matches the registered one
    ensure!(
        validate_redirect_uri(&app.redirect_uri, &data.redirect_uri),
        InvalidClientSnafu
    );

    // Generate oauth_code object to be finalized later at token generation
    let code = generate_id("oac");

    let new_code = NewOauthCodeDto {
        code: code.clone(),
        state: data.state.clone(),
        redirect_uri: data.redirect_uri,
        scope: data.scope,
        app_id: app.id,
        org_id: actor_dto.org_id,
        user_id: actor_dto.id,
    };

    create_oauth_code_svc(&state, new_code).await?;

    let buffed_auth_code = OauthAuthorizationCodeBuf {
        code: code.clone(),
        state: data.state,
    };

    Ok(build_response(200, buffed_auth_code.encode_to_vec()))
}
