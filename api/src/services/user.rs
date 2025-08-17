use serde::Deserialize;
use snafu::{OptionExt, ResultExt, ensure};
use validator::Validate;

use crate::error::{DbSnafu, ValidationSnafu, WhateverSnafu};
use crate::state::AppState;
use crate::{Error, Result};
use db::user::{NewUser, UpdateUser};
use yaas::dto::UserDto;
use yaas::validators::flatten_errors;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct NewUserDto {
    #[validate(email)]
    #[validate(length(min = 1, max = 250))]
    pub email: String,

    #[validate(length(min = 1, max = 100))]
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateUserDto {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,

    #[validate(custom(function = "yaas::validators::status"))]
    pub status: Option<String>,

    pub updated_at: Option<String>,
}

pub async fn create_user(state: &AppState, data: &NewUser, is_setup: bool) -> Result<UserDto> {
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    // Email must be unique
    let existing = state
        .db
        .users
        .find_by_email(&data.email)
        .await
        .context(DbSnafu)?;

    ensure!(
        existing.is_none(),
        ValidationSnafu {
            msg: "Email already exists".to_string(),
        }
    );

    state.db.users.create(data).await.context(DbSnafu)
}

pub async fn update_user_status(state: &AppState, id: &str, data: &UpdateUser) -> Result<bool> {
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    if data.status.is_none() || data.name.is_none() {
        return Ok(false);
    }

    if let Some(user_status) = &data.status {
        ensure!(
            user_status == "active" || user_status == "inactive",
            ValidationSnafu {
                msg: "User status must be active or inactive",
            }
        );
    }

    state.db.users.update(id, data).await.context(DbSnafu)
}
