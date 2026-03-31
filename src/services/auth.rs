use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ensure};

use crate::dto::{
    Actor, ActorPayloadDto, AuthResponseDto, CredentialsDto, ListingParamsDto, Scope,
    SwitchAuthContextDto,
};
use crate::error::{
    ForbiddenSnafu, InactiveUserSnafu, InvalidClientSnafu, InvalidPasswordSnafu, UserNoOrgSnafu,
    UserNotFoundSnafu, WhateverSnafu,
};
use crate::services::password::verify_password;
use crate::services::token::{create_auth_token, verify_auth_token};
use crate::{Result, run::AppState};

pub async fn authenticate(
    state: &AppState,
    credentials: &CredentialsDto,
) -> Result<AuthResponseDto> {
    // Validate user
    let user = state
        .db
        .users
        .find_by_email(credentials.email.clone())
        .await?;

    let user = user.context(InvalidPasswordSnafu)?;

    ensure!(&user.status == "active", InactiveUserSnafu);

    // Validate password
    let passwd = state
        .db
        .passwords
        .get(user.id.clone())
        .await?
        .context(WhateverSnafu {
            msg: "User does not have a password set".to_string(),
        })?;

    let valid = verify_password(&credentials.password, &passwd.password)?;
    ensure!(valid, InvalidPasswordSnafu);

    let user_id = user.id.clone();

    // Check for org memberships
    let org_listing = state
        .db
        .org_members
        .list_memberships(
            user_id.clone(),
            ListingParamsDto {
                page: Some(1),
                per_page: Some(1),
            },
        )
        .await?;

    ensure!(org_listing.meta.total_records > 0, UserNoOrgSnafu);

    // Select the first org, just let the user switch in the frontend
    let org_id = org_listing.data[0].org_id.clone();
    let actor = ActorPayloadDto {
        id: user_id,
        org_id: org_id.clone(),
        org_count: org_listing.meta.total_records as i32,
        roles: org_listing.data[0].roles.clone(),
        scopes: vec![Scope::Auth],
    };

    let token = create_auth_token(&actor, &state.config.jwt_secret)?;

    Ok(AuthResponseDto {
        user,
        token,
        org_id,
        org_count: org_listing.meta.total_records as i32,
    })
}

pub async fn authenticate_token_svc(state: &AppState, token: &str) -> Result<Actor> {
    let actor_payload = verify_auth_token(token, &state.config.jwt_secret)?;
    let user_id = actor_payload.id.clone();
    let org_id = actor_payload.org_id.clone();

    // If found in cache, return right away
    if let Some(cached_actor) = state.auth_cache.get(&user_id) {
        return Ok(cached_actor.clone());
    }

    // Validate org
    let org = state.db.orgs.get(org_id).await?;
    let _ = org.context(InvalidClientSnafu)?;

    let user = state.db.users.get(user_id.clone()).await?;
    let user = user.context(UserNotFoundSnafu)?;

    let actor = Actor::new(actor_payload, user.clone());

    // Store to cache
    state.auth_cache.insert(user_id, actor.clone());

    Ok(actor)
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SwitchAuthContextFormData {
    pub token: String,
    pub org_id: String,
    pub org_name: String,
    pub next: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SwitchAuthContextParams {
    pub org_id: String,
    pub org_name: String,
    pub next: String,
}

pub async fn switch_auth_context_svc(
    state: &AppState,
    user_id: &str,
    payload: SwitchAuthContextDto,
) -> Result<AuthResponseDto> {
    let user_id = user_id.to_owned();
    let org_id = payload.org_id.clone();

    // Validate org membership
    let membership = state
        .db
        .org_members
        .find_member(org_id.clone(), user_id.clone())
        .await?;

    let membership = membership.context(ForbiddenSnafu {
        msg: "User must be a member of the org".to_string(),
    })?;

    // Refresh user info
    let user = state.db.users.get(user_id.clone()).await?;
    let user = user.context(WhateverSnafu {
        msg: "Unable to reload user info".to_string(),
    })?;

    // Refresh org count
    let org_count = state.db.org_members.list_memberships_count(user_id).await?;

    // Switch to the new org
    let actor = ActorPayloadDto {
        id: user.id.clone(),
        org_id: org_id.clone(),
        org_count: org_count as i32,
        roles: membership.roles,
        scopes: vec![Scope::Auth],
    };

    let token = create_auth_token(&actor, &state.config.jwt_secret)?;

    Ok(AuthResponseDto {
        user,
        token,
        org_id,
        org_count: org_count as i32,
    })
}
