use serde::Deserialize;
use snafu::{ResultExt, ensure};
use validator::Validate;

use crate::Result;
use crate::error::{DbSnafu, ValidationSnafu};
use crate::state::AppState;
use db::password::{NewPassword, UpdatePassword};
use yaas::dto::PasswordDto;
use yaas::validators::flatten_errors;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct NewPasswordDto {
    #[validate(length(min = 8, max = 60))]
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdatePasswordDto {
    #[validate(length(min = 8, max = 60))]
    pub password: Option<String>,
}

pub async fn create_password(
    state: &AppState,
    user_id: i32,
    data: &NewPasswordDto,
) -> Result<PasswordDto> {
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let insert_data = NewPassword {
        password: data.password.clone(),
    };

    state
        .db
        .passwords
        .create(user_id, &insert_data)
        .await
        .context(DbSnafu)
}

pub async fn update_password(
    state: &AppState,
    user_id: i32,
    data: &UpdatePasswordDto,
) -> Result<bool> {
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    if data.password.is_none() {
        return Ok(false);
    }

    let update_data = UpdatePassword {
        password: data.password.clone(),
        updated_at: Some(chrono::Utc::now()),
    };

    state
        .db
        .passwords
        .update(user_id, &update_data)
        .await
        .context(DbSnafu)
}

pub async fn delete_password(state: &AppState, id: i32) -> Result<()> {
    state.db.passwords.delete(id).await.context(DbSnafu)
}
