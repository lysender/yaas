use snafu::ResultExt;
use validator::Validate;

use crate::token::{create_auth_token, verify_auth_token};
use password::verify_password;
use snafu::{OptionExt, ensure};
use yaas::actor::{Actor, ActorPayload, AuthResponse, Credentials};

use crate::error::{
    DbSnafu, InactiveUserSnafu, InvalidClientSnafu, InvalidPasswordSnafu, PasswordSnafu,
    UserNotFoundSnafu, ValidationSnafu, WhateverSnafu,
};
use crate::{Result, state::AppState};
use yaas::validators::flatten_errors;

pub async fn authenticate(state: &AppState, credentials: &Credentials) -> Result<AuthResponse> {
    let errors = credentials.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    // Validate user
    let user = state
        .db
        .users
        .find_by_email(&credentials.username)
        .await
        .context(DbSnafu)?;

    let user = user.context(InvalidPasswordSnafu)?;

    ensure!(&user.status == "active", InactiveUserSnafu);

    // Validate password
    let passwd = state
        .db
        .passwords
        .get(&user.id)
        .await
        .context(DbSnafu)?
        .context(WhateverSnafu {
            msg: "User does not have a password set".to_string(),
        })?;

    let _ = verify_password(&credentials.password, &passwd.password).context(PasswordSnafu)?;

    // Generate a token
    // TODO: What to do with orgs?
    let actor = ActorPayload {
        id: user.id.clone(),
        org_id: "".to_string(),
        scope: "auth vault".to_string(),
    };

    let token = create_auth_token(&actor, &state.config.jwt_secret)?;
    Ok(AuthResponse {
        user: user.into(),
        token,
    })
}

pub async fn authenticate_token(state: &AppState, token: &str) -> Result<Actor> {
    let actor = verify_auth_token(token, &state.config.jwt_secret)?;

    // TODO: What to do with orgs?
    // Validate org
    let org = state.db.orgs.get(&actor.org_id).await.context(DbSnafu)?;
    let org = org.context(InvalidClientSnafu)?;

    let user = state.db.users.get(&actor.id).await.context(DbSnafu)?;
    let user = user.context(UserNotFoundSnafu)?;

    Ok(Actor::new(actor, user.into()))
}
