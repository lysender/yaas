use password::hash_password;
use snafu::{ResultExt, ensure};
use yaas::pagination::Paginated;

use crate::Result;
use crate::error::{DbSnafu, PasswordSnafu, ValidationSnafu};
use crate::state::AppState;
use yaas::dto::{ListUsersParamsDto, NewUserWithPasswordDto, UpdateUserDto, UserDto};

pub async fn list_users_svc(
    state: &AppState,
    params: ListUsersParamsDto,
) -> Result<Paginated<UserDto>> {
    state.db.users.list(params).await.context(DbSnafu)
}

pub async fn create_user_svc(
    state: &AppState,
    mut data: NewUserWithPasswordDto,
) -> Result<UserDto> {
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

    // Hash password before sending to DB
    data.password = hash_password(&data.password).context(PasswordSnafu)?;

    state
        .db
        .users
        .create_with_password(data)
        .await
        .context(DbSnafu)
}

pub async fn get_user_svc(state: &AppState, id: i32) -> Result<Option<UserDto>> {
    state.db.users.get(id).await.context(DbSnafu)
}

pub async fn update_user_svc(state: &AppState, id: i32, data: UpdateUserDto) -> Result<bool> {
    state.db.users.update(id, data).await.context(DbSnafu)
}

pub async fn delete_user_svc(state: &AppState, id: i32) -> Result<bool> {
    // Delete user and password
    let deleted = state.db.users.delete(id).await.context(DbSnafu)?;

    // No need to wrap in a transaction, who cares if delete of password fails
    state.db.passwords.delete(id).await.context(DbSnafu)?;

    Ok(deleted)
}
