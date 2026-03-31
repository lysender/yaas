use snafu::{OptionExt, ensure};

use crate::ctx::Ctx;
use crate::dto::{
    ActorPayloadDto, NewOauthCodeDto, OauthAuthorizationCodeDto, OauthAuthorizeDto,
    OauthClientAppDto, OauthClientLookupDto, OauthTokenRequestDto, OauthTokenResponseDto, Scope,
    to_scopes,
};
use crate::error::{
    AppNotRegisteredSnafu, ForbiddenSnafu, InvalidClientSnafu, OauthCodeInvalidSnafu,
    OauthInvalidScopesSnafu, OauthStateMismatchSnafu, RedirectUriMistmatchSnafu,
};
use crate::run::AppState;
use crate::services::oauth_code::{create_oauth_code_svc, delete_oauth_code_svc};
use crate::services::token::create_auth_token;
use crate::utils::{IdPrefix, generate_id, validate_redirect_uri};
use crate::{Error, Result};

pub async fn create_authorization_code_svc(
    state: &AppState,
    ctx: &Ctx,
    query: &OauthAuthorizeDto,
) -> Result<OauthAuthorizationCodeDto> {
    // Validate scopes
    let scope_list: Vec<String> = query
        .scope
        .split(' ')
        .filter(|scope| !scope.is_empty())
        .map(|scope| scope.to_string())
        .collect();

    // Convert it to scope enums
    let scopes = to_scopes(&scope_list)?;

    // Allowed scopes
    let allowed_scopes = [Scope::Auth, Scope::Oauth];

    // Ensure all requested scopes are allowed
    let invalid_scopes = scopes
        .iter()
        .filter(|s| !allowed_scopes.contains(s))
        .count();

    ensure!(invalid_scopes == 0, InvalidClientSnafu);

    let actor = ctx.actor().context(InvalidClientSnafu)?;

    // Validate client_id from the apps table
    let app = state
        .db
        .apps
        .find_by_client_id(query.client_id.clone())
        .await?;

    let app = app.context(InvalidClientSnafu)?;
    let app_id = app.id.clone();
    let actor_org_id = actor.org_id.clone();
    let actor_user_id = actor.id.clone();

    // Ensure redirect_uri is valid and matches the registered one
    ensure!(
        validate_redirect_uri(&app.redirect_uri, &query.redirect_uri),
        RedirectUriMistmatchSnafu
    );

    // Ensure that the app is registered to the user's current org
    let org_app = state
        .db
        .org_apps
        .find_app(actor_org_id.clone(), app_id.clone())
        .await?;

    ensure!(org_app.is_some(), AppNotRegisteredSnafu);

    // Generate oauth_code object to be finalized later at token generation
    let code = generate_id(IdPrefix::OauthCode);

    let new_code = NewOauthCodeDto {
        code: code.clone(),
        state: query.state.clone(),
        redirect_uri: query.redirect_uri.clone(),
        scope: query.scope.clone(),
        app_id,
        org_id: actor_org_id,
        user_id: actor_user_id,
    };

    create_oauth_code_svc(&state, new_code).await?;

    let auth_code = OauthAuthorizationCodeDto {
        code: code.clone(),
        state: query.state.clone(),
    };

    Ok(auth_code)
}

pub async fn exchange_code_for_access_token_svc(
    state: &AppState,
    payload: &OauthTokenRequestDto,
) -> Result<OauthTokenResponseDto> {
    // Find the authorization code
    let oauth_code = state.db.oauth_codes.find_by_code(&payload.code).await?;

    let oauth_code = oauth_code.context(OauthCodeInvalidSnafu)?;
    let oauth_org_id = oauth_code.org_id.clone();
    let oauth_user_id = oauth_code.user_id.clone();

    // Ensure that parameters match those used during authorization
    ensure!(oauth_code.state == payload.state, OauthStateMismatchSnafu);
    ensure!(
        oauth_code.redirect_uri == payload.redirect_uri,
        RedirectUriMistmatchSnafu
    );

    // Validate client_id and client_secret
    let app = state
        .db
        .apps
        .find_by_client_id(payload.client_id.clone())
        .await?;

    let app = app.context(InvalidClientSnafu)?;

    ensure!(
        app.client_secret == payload.client_secret,
        InvalidClientSnafu
    );

    // Parse scopes
    let scope_list: Vec<String> = oauth_code
        .scope
        .split(' ')
        .filter(|scope| !scope.is_empty())
        .map(|scope| scope.to_string())
        .collect();

    let scopes = to_scopes(&scope_list)?;

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
        .await?;

    let membership = membership.context(ForbiddenSnafu {
        msg: "User must be a member of the org".to_string(),
    })?;

    // Count org memberships for the user
    let org_count = state
        .db
        .org_members
        .list_memberships_count(oauth_user_id.clone())
        .await?;

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

    Ok(response)
}

pub async fn lookup_oauth_client_app_svc(
    state: &AppState,
    payload: &OauthClientLookupDto,
) -> Result<OauthClientAppDto> {
    // Find app by client_id
    let Some(app) = state
        .db
        .apps
        .find_by_client_id(payload.client_id.to_owned())
        .await?
    else {
        return Err(Error::InvalidClient);
    };

    // Validate if redirect_uri is valid
    ensure!(
        validate_redirect_uri(&app.redirect_uri, &payload.redirect_uri),
        InvalidClientSnafu
    );

    Ok(OauthClientAppDto { name: app.name })
}
