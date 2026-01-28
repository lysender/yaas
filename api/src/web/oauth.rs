use axum::{
    Extension, Router,
    body::{Body, Bytes},
    extract::State,
    middleware,
    response::Response,
    routing::post,
};
use prost::Message;
use snafu::{OptionExt, ResultExt, ensure};
use validator::Validate;

use crate::{
    Error, Result,
    error::{DbSnafu, ForbiddenSnafu, InvalidClientSnafu, InvalidScopesSnafu, ValidationSnafu},
    services::oauth_code::{create_oauth_code_svc, delete_oauth_code_svc},
    state::AppState,
    token::create_auth_token,
    web::{
        build_response,
        middleware::{auth_middleware, require_auth_middleware},
    },
};
use yaas::{
    buffed::dto::{
        OauthAuthorizationCodeBuf, OauthAuthorizeBuf, OauthTokenRequestBuf, OauthTokenResponseBuf,
    },
    dto::{Actor, ActorPayloadDto, NewOauthCodeDto, OauthAuthorizeDto, OauthTokenRequestDto},
    role::to_scopes,
    utils::{generate_id, validate_redirect_uri},
    validators::flatten_errors,
};

pub fn oauth_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(oauth_authorize_route(state.clone()))
        .merge(oauth_token_route(state.clone()))
        .with_state(state)
}

fn oauth_authorize_route(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/oauth/authorize", post(oauth_authorize_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            require_auth_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .with_state(state)
}

fn oauth_token_route(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/oauth/token", post(oauth_token_handler))
        .with_state(state)
}

/// POST /oauth/authorize
/// Initiates OAuth2 authorization code flow
/// Requires authenticated user
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
        .context(DbSnafu)?;

    let app = app.context(InvalidClientSnafu)?;

    // Ensure redirect_uri is valid and matches the registered one
    ensure!(
        validate_redirect_uri(&app.redirect_uri, &data.redirect_uri),
        InvalidClientSnafu
    );

    // Ensure that the app is registered to the user's current org
    let org_app = state
        .db
        .org_apps
        .find_app(actor_dto.org_id, app.id)
        .await
        .context(DbSnafu)?;

    ensure!(org_app.is_some(), InvalidClientSnafu);

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

/// POST /oauth/token
/// Exchanges an OAuth authorization code for an access token
async fn oauth_token_handler(State(state): State<AppState>, body: Bytes) -> Result<Response<Body>> {
    let Ok(payload) = OauthTokenRequestBuf::decode(body) else {
        return Err(Error::BadProtobuf);
    };

    let data: OauthTokenRequestDto = payload.into();
    let errors = data.validate();

    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    // Find the authorization code
    let oauth_code = state
        .db
        .oauth_codes
        .find_by_code(&data.code)
        .await
        .context(DbSnafu)?;

    let oauth_code = oauth_code.context(InvalidClientSnafu)?;

    // Ensure that parameters match those used during authorization
    ensure!(oauth_code.state == data.state, InvalidClientSnafu);
    ensure!(
        oauth_code.redirect_uri == data.redirect_uri,
        InvalidClientSnafu
    );

    // Validate client_id and client_secret
    let app = state
        .db
        .apps
        .find_by_client_id(&data.client_id)
        .await
        .context(DbSnafu)?;

    let app = app.context(InvalidClientSnafu)?;

    ensure!(app.client_secret == data.client_secret, InvalidClientSnafu);

    // Parse scopes
    let scope_list: Vec<String> = oauth_code
        .scope
        .split(' ')
        .filter(|scope| !scope.is_empty())
        .map(|scope| scope.to_string())
        .collect();

    let scopes = to_scopes(&scope_list).context(InvalidScopesSnafu)?;

    // Fetch roles for the user in the org
    let membership = state
        .db
        .org_members
        .find_member(oauth_code.org_id, oauth_code.user_id)
        .await
        .context(DbSnafu)?;

    let membership = membership.context(ForbiddenSnafu {
        msg: "User must be a member of the org".to_string(),
    })?;

    // Count org memberships for the user
    let org_count = state
        .db
        .org_members
        .list_memberships_count(oauth_code.user_id)
        .await
        .context(DbSnafu)?;

    // Create a token
    let payload = ActorPayloadDto {
        id: oauth_code.user_id,
        org_id: oauth_code.org_id,
        org_count: org_count as i32,
        roles: membership.roles.clone(),
        scopes,
    };

    let token = create_auth_token(&payload, &state.config.jwt_secret)?;

    // Cleanup oauth code so it cannot be used again
    delete_oauth_code_svc(&state, oauth_code.id).await?;

    let buffed_response = OauthTokenResponseBuf {
        access_token: token,
        scope: oauth_code.scope,
        token_type: "app".to_string(),
    };

    Ok(build_response(200, buffed_response.encode_to_vec()))
}
