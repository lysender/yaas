use argon2::{
    Argon2,
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString, rand_core::OsRng},
};
use snafu::{OptionExt, ensure};

use crate::run::AppState;
use crate::{Result, services::users::ChangeCurrentPasswordFormData};
use crate::{
    dto::{ChangeCurrentPasswordDto, NewPasswordDto},
    services::token::verify_csrf_token,
};
use crate::{
    error::{
        CsrfTokenSnafu, HashPasswordSnafu, ValidationSnafu, VerifyPasswordHashSnafu, WhateverSnafu,
    },
    services::users::ChangePasswordFormData,
};

pub async fn update_password_svc(
    state: &AppState,
    user_id: &str,
    data: NewPasswordDto,
) -> Result<bool> {
    let hashed_password = hash_password(&data.password)?;
    let updated_data = NewPasswordDto {
        password: hashed_password,
    };

    state
        .db
        .passwords
        .update(user_id.to_string(), updated_data)
        .await
}

pub async fn change_user_password_web_svc(
    state: &AppState,
    user_id: &str,
    form: ChangePasswordFormData,
) -> Result<()> {
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == user_id, CsrfTokenSnafu);

    ensure!(
        form.password == form.confirm_password,
        ValidationSnafu {
            msg: "Passwords must match."
        }
    );

    let body = NewPasswordDto {
        password: form.password,
    };

    update_password_svc(state, user_id, body).await?;

    Ok(())
}

pub async fn change_current_password_svc(
    state: &AppState,
    user_id: &str,
    data: ChangeCurrentPasswordDto,
) -> Result<bool> {
    // Validate current password
    let password = state
        .db
        .passwords
        .get(user_id.to_string())
        .await?
        .context(WhateverSnafu {
            msg: "User has no password set".to_string(),
        })?;

    let valid = verify_password(&data.current_password, &password.password)?;

    ensure!(
        valid,
        ValidationSnafu {
            msg: "Current password is incorrect".to_string(),
        }
    );

    let hashed_password = hash_password(&data.new_password)?;

    // Update password
    let update_data = NewPasswordDto {
        password: hashed_password,
    };

    state
        .db
        .passwords
        .update(user_id.to_string(), update_data)
        .await
}

pub async fn change_user_current_password_web_svc(
    state: &AppState,
    user_id: &str,
    form: ChangeCurrentPasswordFormData,
) -> Result<()> {
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == user_id, CsrfTokenSnafu);

    ensure!(
        form.new_password == form.confirm_new_password,
        ValidationSnafu {
            msg: "Passwords must match."
        }
    );

    let body = ChangeCurrentPasswordDto {
        current_password: form.current_password,
        new_password: form.new_password,
    };

    change_current_password_svc(state, user_id, body).await?;

    Ok(())
}

pub fn hash_password(password: &str) -> Result<String> {
    let pwd = password.as_bytes();
    let salt = SaltString::generate(&mut OsRng);
    let gon = Argon2::default();
    match gon.hash_password(pwd, &salt) {
        Ok(hash) => Ok(hash.to_string()),
        Err(e) => HashPasswordSnafu {
            msg: format!("Error hashing password: {}", e),
        }
        .fail(),
    }
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool> {
    let Ok(parsed_hash) = PasswordHash::new(hash) else {
        return VerifyPasswordHashSnafu {
            msg: "Invalid password hash".to_string(),
        }
        .fail();
    };
    let gone = Argon2::default();
    match gone.verify_password(password.as_bytes(), &parsed_hash) {
        Ok(_) => Ok(true),
        Err(_) => Ok(false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password() {
        let password = "password";
        let hash = hash_password(password).unwrap();
        assert!(hash.len() > 0);
    }

    #[test]
    fn test_verify_password() {
        let password = "password";
        let stored_hash = "$argon2id$v=19$m=19456,t=2,p=1$NxAcor94oNDtRqstYqRvmA$EtLJjVFPFz0hE5QLZ/ydx4Td4slp9GaXuwQX3vQU9Dc";

        let result = verify_password(password, &stored_hash).unwrap();
        assert!(result);

        // Try again
        let result = verify_password(password, &stored_hash).unwrap();
        assert!(result);
    }
}
