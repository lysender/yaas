use snafu::ResultExt;
use snafu::{OptionExt, ensure};
use yaas::pagination::ListingParamsDto;

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
    let org_listing = state
        .db
        .org_members
        .list_memberships(
            user.id,
            ListingParamsDto {
                page: Some(1),
                per_page: Some(1),
            },
        )
        .await
        .context(DbSnafu)?;

    ensure!(org_listing.meta.total_records > 0, UserNoOrgSnafu);

    // Select the first org, just let the user switch in the frontend
    let actor = ActorPayload {
        id: user.id,
        org_id: org_listing.data[0].org_id,
        org_count: org_listing.meta.total_records as i32,
        roles: org_listing.data[0].roles.clone(),
        scope: "auth org".to_string(),
    };

    let token = create_auth_token(&actor, &state.config.jwt_secret)?;

    Ok(AuthResponse {
        user: user.into(),
        token,
        org_id: org_listing.data[0].org_id,
        org_count: org_listing.meta.total_records as i32,
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
