use password::{hash_password, verify_password};
use snafu::{OptionExt, ResultExt, ensure};

use crate::Result;
use crate::error::{DbSnafu, PasswordSnafu, ValidationSnafu, WhateverSnafu};
use crate::state::AppState;
use yaas::dto::{ChangeCurrentPasswordDto, NewPasswordDto, UpdatePasswordDto};

pub async fn create_password_svc(
    state: &AppState,
    user_id: i32,
    data: NewPasswordDto,
) -> Result<()> {
    let new_password = hash_password(&data.password).context(PasswordSnafu)?;

    // Create a new password object with a hashed password
    // Password length is not validated here anymore
    let insert_data = NewPasswordDto {
        password: new_password,
    };

    state
        .db
        .passwords
        .create(user_id, insert_data)
        .await
        .context(DbSnafu)
}

pub async fn update_password_svc(
    state: &AppState,
    user_id: i32,
    data: UpdatePasswordDto,
) -> Result<bool> {
    if data.password.is_none() {
        return Ok(false);
    }

    state
        .db
        .passwords
        .update(user_id, data)
        .await
        .context(DbSnafu)
}

pub async fn change_current_password_svc(
    state: &AppState,
    user_id: i32,
    data: ChangeCurrentPasswordDto,
) -> Result<bool> {
    // Validate current password
    let password = state
        .db
        .passwords
        .get(user_id)
        .await
        .context(DbSnafu)?
        .context(WhateverSnafu {
            msg: "User has no password set".to_string(),
        })?;

    let valid =
        verify_password(&data.current_password, &password.password).context(PasswordSnafu)?;

    ensure!(
        valid,
        ValidationSnafu {
            msg: "Current password is incorrect".to_string(),
        }
    );

    let hashed_password = hash_password(&data.new_password).context(PasswordSnafu)?;

    // Update password
    let update_data = UpdatePasswordDto {
        password: Some(hashed_password),
    };

    state
        .db
        .passwords
        .update(user_id, update_data)
        .await
        .context(DbSnafu)
}

pub async fn delete_password_svc(state: &AppState, id: i32) -> Result<()> {
    state.db.passwords.delete(id).await.context(DbSnafu)
}
