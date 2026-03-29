use axum::{
    Extension, Router, body::Body, extract::State, http::StatusCode, middleware,
    response::Response, routing::post,
};
use snafu::{OptionExt, ResultExt, ensure};

use crate::{
    Result,
    error::{
        AppNotRegisteredSnafu, DbSnafu, ForbiddenSnafu, InvalidClientSnafu, InvalidScopesSnafu,
        OauthCodeInvalidSnafu, OauthInvalidScopesSnafu, OauthStateMismatchSnafu,
        RedirectUriMistmatchSnafu,
    },
    services::oauth_code::{create_oauth_code_svc, delete_oauth_code_svc},
    state::AppState,
    token::create_auth_token,
    web::{
        json_input::{JsonPayload, validate_json_payload},
        json_response,
        middleware::{auth_middleware, require_auth_middleware},
    },
};
use yaas::{
    dto::{
        Actor, ActorPayloadDto, NewOauthCodeDto, OauthAuthorizationCodeDto, OauthAuthorizeDto,
        OauthClientAppDto, OauthClientLookupDto, OauthTokenRequestDto, OauthTokenResponseDto,
    },
    role::{Scope, to_scopes},
    utils::{IdPrefix, generate_id, validate_redirect_uri},
};

pub fn oauth_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .merge(oauth_client_lookup_route(state.clone()))
        .merge(oauth_authorize_route(state.clone()))
        .merge(oauth_token_route(state.clone()))
        .with_state(state)
}

fn oauth_client_lookup_route(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/oauth/client", post(oauth_client_lookup_handler))
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

/// POST /oauth/client
/// Validate oauth client_id + redirect_uri and return app name
async fn oauth_client_lookup_handler(
    State(state): State<AppState>,
    payload: JsonPayload<OauthClientLookupDto>,
) -> Result<Response<Body>> {
    let data = validate_json_payload(payload)?;

    let app = state
        .db
        .apps
        .find_by_client_id(data.client_id)
        .await
        .context(DbSnafu)?;

    let app = app.context(InvalidClientSnafu)?;

    ensure!(
        validate_redirect_uri(&app.redirect_uri, &data.redirect_uri),
        InvalidClientSnafu
    );

    Ok(json_response(
        StatusCode::OK,
        OauthClientAppDto { name: app.name },
    ))
}

/// POST /oauth/authorize
/// Initiates OAuth2 authorization code flow
/// Requires authenticated user
async fn oauth_authorize_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    payload: JsonPayload<OauthAuthorizeDto>,
) -> Result<Response<Body>> {
    let data = validate_json_payload(payload)?;

    // Validate scopes
    let scope_list: Vec<String> = data
        .scope
        .split(' ')
        .filter(|scope| !scope.is_empty())
        .map(|scope| scope.to_string())
        .collect();

    // Convert it to scope enums
    let scopes = to_scopes(&scope_list).context(InvalidScopesSnafu)?;

    // Allowed scopes
    let allowed_scopes = [Scope::Auth, Scope::Oauth];

    // Ensure all requested scopes are allowed
    let invalid_scopes = scopes
        .iter()
        .filter(|s| !allowed_scopes.contains(s))
        .count();

    ensure!(invalid_scopes == 0, InvalidClientSnafu);

    let actor_dto = actor.actor.context(InvalidClientSnafu)?;

    // Validate client_id from the apps table
    let app = state
        .db
        .apps
        .find_by_client_id(data.client_id.clone())
        .await
        .context(DbSnafu)?;

    let app = app.context(InvalidClientSnafu)?;
    let app_id = app.id.clone();
    let actor_org_id = actor_dto.org_id.clone();
    let actor_user_id = actor_dto.id.clone();

    // Ensure redirect_uri is valid and matches the registered one
    ensure!(
        validate_redirect_uri(&app.redirect_uri, &data.redirect_uri),
        RedirectUriMistmatchSnafu
    );

    // Ensure that the app is registered to the user's current org
    let org_app = state
        .db
        .org_apps
        .find_app(actor_org_id.clone(), app_id.clone())
        .await
        .context(DbSnafu)?;

    ensure!(org_app.is_some(), AppNotRegisteredSnafu);

    // Generate oauth_code object to be finalized later at token generation
    let code = generate_id(IdPrefix::OauthCode);

    let new_code = NewOauthCodeDto {
        code: code.clone(),
        state: data.state.clone(),
        redirect_uri: data.redirect_uri,
        scope: data.scope,
        app_id,
        org_id: actor_org_id,
        user_id: actor_user_id,
    };

    create_oauth_code_svc(&state, new_code).await?;

    let auth_code = OauthAuthorizationCodeDto {
        code: code.clone(),
        state: data.state,
    };

    Ok(json_response(StatusCode::OK, auth_code))
}

/// POST /oauth/token
/// Exchanges an OAuth authorization code for an access token
async fn oauth_token_handler(
    State(state): State<AppState>,
    payload: JsonPayload<OauthTokenRequestDto>,
) -> Result<Response<Body>> {
    let data = validate_json_payload(payload)?;

    // Find the authorization code
    let oauth_code = state
        .db
        .oauth_codes
        .find_by_code(&data.code)
        .await
        .context(DbSnafu)?;

    let oauth_code = oauth_code.context(OauthCodeInvalidSnafu)?;
    let oauth_org_id = oauth_code.org_id.clone();
    let oauth_user_id = oauth_code.user_id.clone();

    // Ensure that parameters match those used during authorization
    ensure!(oauth_code.state == data.state, OauthStateMismatchSnafu);
    ensure!(
        oauth_code.redirect_uri == data.redirect_uri,
        RedirectUriMistmatchSnafu
    );

    // Validate client_id and client_secret
    let app = state
        .db
        .apps
        .find_by_client_id(data.client_id.clone())
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

    // Allowed scopes
    let allowed_scopes = [Scope::Auth, Scope::Oauth];

    // Ensure all requested scopes are allowed
    let invalid_scopes: Vec<&Scope> = scopes
        .iter()
        .filter(|s| !allowed_scopes.contains(s))
        .collect();

    ensure!(invalid_scopes.is_empty(), OauthInvalidScopesSnafu);

    // Fetch roles for the user in the org
    let membership = state
        .db
        .org_members
        .find_member(oauth_org_id.clone(), oauth_user_id.clone())
        .await
        .context(DbSnafu)?;

    let membership = membership.context(ForbiddenSnafu {
        msg: "User must be a member of the org".to_string(),
    })?;

    // Count org memberships for the user
    let org_count = state
        .db
        .org_members
        .list_memberships_count(oauth_user_id.clone())
        .await
        .context(DbSnafu)?;

    // Create a token
    let payload = ActorPayloadDto {
        id: oauth_user_id,
        org_id: oauth_org_id,
        org_count: org_count as i32,
        roles: membership.roles.clone(),
        scopes,
    };

    let token = create_auth_token(&payload, &state.config.jwt_secret)?;

    // Cleanup oauth code so it cannot be used again
    delete_oauth_code_svc(&state, &oauth_code.id).await?;

    let response = OauthTokenResponseDto {
        access_token: token,
        scope: oauth_code.scope,
        token_type: "app".to_string(),
    };

    Ok(json_response(StatusCode::OK, response))
}
