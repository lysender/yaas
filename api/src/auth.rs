use snafu::ResultExt;
use snafu::{OptionExt, ensure};

use crate::error::{
    DbSnafu, InactiveUserSnafu, InvalidClientSnafu, InvalidPasswordSnafu, PasswordSnafu,
    UserNoOrgSnafu, UserNotFoundSnafu, WhateverSnafu,
};
use crate::token::{create_auth_token, verify_auth_token};
use crate::{Result, state::AppState};
use password::verify_password;
use yaas::actor::{Actor, ActorPayload, AuthResponse, Credentials};

/// Authenticates a user with the provided credentials.
pub async fn authenticate(state: &AppState, credentials: &Credentials) -> Result<AuthResponse> {
    // Validate user
    let user = state
        .db
        .users
        .find_by_email(&credentials.email)
        .await
        .context(DbSnafu)?;

    let user = user.context(InvalidPasswordSnafu)?;

    ensure!(&user.status == "active", InactiveUserSnafu);

    // Validate password
    let passwd = state
        .db
        .passwords
        .get(user.id)
        .await
        .context(DbSnafu)?
        .context(WhateverSnafu {
            msg: "User does not have a password set".to_string(),
        })?;

    let valid = verify_password(&credentials.password, &passwd.password).context(PasswordSnafu)?;
    ensure!(valid, InvalidPasswordSnafu);

    // Check for org memberships
    let orgs = state
        .db
        .org_members
        .list_memberships(user.id)
        .await
        .context(DbSnafu)?;

    ensure!(orgs.len() > 0, UserNoOrgSnafu);

    if orgs.len() == 1 {
        // We're good to go, select the org and create a token
        let actor = ActorPayload {
            id: user.id.clone(),
            org_id: orgs[0].org_id,
            roles: orgs[0].roles.clone(),
            scope: "auth org".to_string(),
        };

        let token = create_auth_token(&actor, &state.config.jwt_secret)?;
        return Ok(AuthResponse {
            user: user.into(),
            token: Some(token),
            select_org_token: None,
            select_org_options: Vec::new(),
        });
    }

    // Let the user select an org first before issuing a proper token
    let actor = ActorPayload {
        id: user.id.clone(),
        org_id: orgs[0].org_id, // org_id is ignored in this case
        roles: Vec::new(),
        scope: "".to_string(), // Not fully authenticated yet
    };

    let token = create_auth_token(&actor, &state.config.jwt_secret)?;
    Ok(AuthResponse {
        user: user.into(),
        token: None,
        select_org_token: Some(token),
        select_org_options: orgs,
    })
}

pub async fn authenticate_token(state: &AppState, token: &str) -> Result<Actor> {
    let actor = verify_auth_token(token, &state.config.jwt_secret)?;

    // If found in cache, return right away
    if let Some(cached_user) = state.auth_cache.get(&actor.id) {
        return Ok(Actor::new(actor, cached_user));
    }

    // Validate org
    let org = state.db.orgs.get(actor.org_id).await.context(DbSnafu)?;
    let _ = org.context(InvalidClientSnafu)?;

    let user = state.db.users.get(actor.id).await.context(DbSnafu)?;
    let user = user.context(UserNotFoundSnafu)?;

    // Store to cache
    state.auth_cache.insert(actor.id, user.clone());

    Ok(Actor::new(actor, user.into()))
}
