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

pub async fn create_user(state: &AppState, data: &NewUserDto) -> Result<UserDto> {
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

    let insert_data = NewUser {
        email: data.email.clone(),
        name: data.name.clone(),
    };

    state.db.users.create(&insert_data).await.context(DbSnafu)
}

pub async fn update_user(state: &AppState, id: &str, data: &UpdateUserDto) -> Result<bool> {
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

    let update_data = UpdateUser {
        name: data.name.clone(),
        status: data.status.clone(),
        updated_at: data.updated_at.clone(),
    };

    state
        .db
        .users
        .update(id, &update_data)
        .await
        .context(DbSnafu)
}
