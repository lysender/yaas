use snafu::{ResultExt, ensure};

use crate::Result;
use crate::error::{DbSnafu, ValidationSnafu};
use crate::state::AppState;
use yaas::dto::{NewUserDto, UpdateUserDto, UserDto};

pub async fn create_user_svc(state: &AppState, data: NewUserDto) -> Result<UserDto> {
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

pub async fn update_user_svc(state: &AppState, id: i32, data: UpdateUserDto) -> Result<bool> {
    if data.status.is_none() || data.name.is_none() {
        return Ok(false);
    }

    state.db.users.update(id, data).await.context(DbSnafu)
}

pub async fn delete_user_svc(state: &AppState, id: i32) -> Result<bool> {
    state.db.users.delete(id).await.context(DbSnafu)
}
