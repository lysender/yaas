use password::hash_password;
use snafu::{ResultExt, ensure};
use validator::Validate;

use crate::Result;
use crate::error::{DbSnafu, PasswordSnafu, ValidationSnafu};
use crate::state::AppState;
use db::password::{NewPassword, UpdatePassword};
use yaas::dto::{NewPasswordDto, PasswordDto, UpdatePasswordDto};
use yaas::validators::flatten_errors;

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

    let new_password = hash_password(&data.password).context(PasswordSnafu)?;

    let insert_data = NewPassword {
        password: new_password,
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
