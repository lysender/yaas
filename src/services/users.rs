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

#[cfg(test)]
mod tests {
    use crate::dto::NewUserWithPasswordDto;
    use crate::services::password::verify_password;
    use crate::services::token::create_csrf_token_svc;
    use crate::test::TestCtx;

    use super::{
        UserActiveFormData, create_user_svc, delete_user_web_svc, get_user_svc,
        update_user_status_web_svc,
    };

    #[tokio::test]
    async fn create_user_saves_and_hashes_password() {
        let ctx = TestCtx::new("users_create_hash").await.expect("test ctx");

        let created = create_user_svc(
            &ctx.state,
            NewUserWithPasswordDto {
                name: "Test User".to_string(),
                email: "test@example.com".to_string(),
                password: "password123".to_string(),
            },
        )
        .await
        .expect("user should be created");

        let fetched = ctx
            .state
            .db
            .users
            .find_by_email("test@example.com".to_string())
            .await
            .expect("query should work")
            .expect("user should exist");

        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.email, "test@example.com");

        let stored_password = ctx
            .state
            .db
            .passwords
            .get(created.id.clone())
            .await
            .expect("password query should work")
            .expect("password should exist");

        assert_ne!(stored_password.password, "password123");

        let valid = verify_password("password123", &stored_password.password)
            .expect("hash should be verifiable");
        assert!(valid);
    }

    #[tokio::test]
    async fn create_user_rejects_duplicate_email() {
        let ctx = TestCtx::new("users_duplicate_email")
            .await
            .expect("test ctx");

        ctx.seed_user_with_password("First User", "dupe@example.com", "password123")
            .await
            .expect("seed should succeed");

        let result = create_user_svc(
            &ctx.state,
            NewUserWithPasswordDto {
                name: "Second User".to_string(),
                email: "dupe@example.com".to_string(),
                password: "password123".to_string(),
            },
        )
        .await;

        assert!(result.is_err());
        let err = result.expect_err("duplicate email should fail");
        assert_eq!(err.to_string(), "Email already exists");
    }

    #[tokio::test]
    async fn update_user_status_web_svc_updates_status_successfully() {
        let ctx = TestCtx::new("users_update_status_success")
            .await
            .expect("test ctx");
        let user = ctx
            .seed_user_with_password("Status User", "status.user@example.com", "password123")
            .await
            .expect("seed user");

        let csrf = create_csrf_token_svc(&user.id, &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");

        let updated = update_user_status_web_svc(
            &ctx.state,
            &user.id,
            UserActiveFormData {
                token: csrf,
                active: None,
            },
        )
        .await
        .expect("status update should pass");

        assert_eq!(updated.id, user.id);
        assert_eq!(updated.status, "inactive");
    }

    #[tokio::test]
    async fn update_user_status_web_svc_rejects_invalid_csrf_token() {
        let ctx = TestCtx::new("users_update_status_invalid_csrf")
            .await
            .expect("test ctx");
        let user = ctx
            .seed_user_with_password("Status User", "status.user.csrf@example.com", "password123")
            .await
            .expect("seed user");

        let result = update_user_status_web_svc(
            &ctx.state,
            &user.id,
            UserActiveFormData {
                token: "invalid.token".to_string(),
                active: None,
            },
        )
        .await;

        assert!(result.is_err(), "invalid csrf should fail");
    }

    #[tokio::test]
    async fn delete_user_web_svc_deletes_user_and_get_returns_none() {
        let ctx = TestCtx::new("users_delete_success")
            .await
            .expect("test ctx");
        let user = ctx
            .seed_user_with_password("Delete User", "delete.user@example.com", "password123")
            .await
            .expect("seed user");

        let csrf = create_csrf_token_svc(&user.id, &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");

        delete_user_web_svc(&ctx.state, &user.id, &csrf)
            .await
            .expect("delete should pass");

        let fetched = get_user_svc(&ctx.state, &user.id)
            .await
            .expect("query should pass");
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn delete_user_web_svc_rejects_invalid_csrf_token() {
        let ctx = TestCtx::new("users_delete_invalid_csrf")
            .await
            .expect("test ctx");
        let user = ctx
            .seed_user_with_password("Delete User", "delete.user.csrf@example.com", "password123")
            .await
            .expect("seed user");

        let result = delete_user_web_svc(&ctx.state, &user.id, "invalid.token").await;

        assert!(result.is_err(), "invalid csrf should fail");
    }
}
