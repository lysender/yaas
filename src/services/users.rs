use serde::{Deserialize, Serialize};
use snafu::ensure;

use crate::dto::Paginated;
use crate::dto::{ListUsersParamsDto, NewUserWithPasswordDto, UpdateUserDto, UserDto};
use crate::error::{CsrfTokenSnafu, ValidationSnafu};
use crate::run::AppState;
use crate::services::password::hash_password;
use crate::services::token::verify_csrf_token;
use crate::{Error, Result};

#[derive(Clone, Deserialize, Serialize)]
pub struct NewUserFormData {
    pub name: String,
    pub email: String,
    pub password: String,
    pub confirm_password: String,
    pub token: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UserActiveFormData {
    pub token: String,
    pub active: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChangeCurrentPasswordFormData {
    pub token: String,
    pub current_password: String,
    pub new_password: String,
    pub confirm_new_password: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChangePasswordFormData {
    pub token: String,
    pub password: String,
    pub confirm_password: String,
}

pub async fn list_users_svc(
    state: &AppState,
    params: ListUsersParamsDto,
) -> Result<Paginated<UserDto>> {
    state.db.users.list(params).await
}

pub async fn create_user_svc(
    state: &AppState,
    mut data: NewUserWithPasswordDto,
) -> Result<UserDto> {
    // Email must be unique
    let existing = state.db.users.find_by_email(data.email.clone()).await?;

    ensure!(
        existing.is_none(),
        ValidationSnafu {
            msg: "Email already exists".to_string(),
        }
    );

    // Hash password before sending to DB
    data.password = hash_password(&data.password)?;

    state.db.users.create_with_password(data).await
}

pub async fn create_user_web_svc(state: &AppState, form: NewUserFormData) -> Result<UserDto> {
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == "new_user", CsrfTokenSnafu);

    ensure!(
        form.password == form.confirm_password,
        ValidationSnafu {
            msg: "Passwords must match".to_string()
        }
    );

    let body = NewUserWithPasswordDto {
        name: form.name,
        email: form.email,
        password: form.password,
    };

    create_user_svc(state, body).await
}

pub async fn get_user_svc(state: &AppState, id: &str) -> Result<Option<UserDto>> {
    state.db.users.get(id.to_string()).await
}

pub async fn update_user_svc(state: &AppState, id: &str, data: UpdateUserDto) -> Result<bool> {
    state.db.users.update(id.to_string(), data).await
}

pub async fn update_user_status_web_svc(
    state: &AppState,
    user_id: &str,
    form: UserActiveFormData,
) -> Result<UserDto> {
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == user_id, CsrfTokenSnafu);

    let body = UpdateUserDto {
        name: None,
        status: match form.active {
            Some(_) => Some("active".to_string()),
            None => Some("inactive".to_string()),
        },
    };

    update_user_svc(state, user_id, body).await?;

    // Fetch the updated user to return
    let Some(updated_user) = get_user_svc(state, user_id).await? else {
        return Err(Error::UserNotFound);
    };

    Ok(updated_user)
}

pub async fn delete_user_svc(state: &AppState, id: &str) -> Result<bool> {
    // Delete user and password
    let deleted = state.db.users.delete(id.to_string()).await?;

    // No need to wrap in a transaction, who cares if delete of password fails
    state.db.passwords.delete(id.to_string()).await?;

    Ok(deleted)
}

pub async fn delete_user_web_svc(state: &AppState, user_id: &str, csrf_token: &str) -> Result<()> {
    let csrf_result = verify_csrf_token(csrf_token, &state.config.jwt_secret)?;
    ensure!(csrf_result == user_id, CsrfTokenSnafu);

    delete_user_svc(state, user_id).await?;

    Ok(())
}
