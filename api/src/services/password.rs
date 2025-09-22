use password::{hash_password, verify_password};
use snafu::{OptionExt, ResultExt, ensure};

use crate::Result;
use crate::error::{DbSnafu, PasswordSnafu, ValidationSnafu, WhateverSnafu};
use crate::state::AppState;
use yaas::dto::{ChangeCurrentPasswordDto, UpdatePasswordDto};

pub async fn update_password_svc(
    state: &AppState,
    user_id: i32,
    data: UpdatePasswordDto,
) -> Result<bool> {
    let hashed_password = hash_password(&data.password).context(PasswordSnafu)?;
    let updated_data = UpdatePasswordDto {
        password: hashed_password,
    };

    state
        .db
        .passwords
        .update(user_id, updated_data)
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
        password: hashed_password,
    };

    state
        .db
        .passwords
        .update(user_id, update_data)
        .await
        .context(DbSnafu)
}
