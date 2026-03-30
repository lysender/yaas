use snafu::ensure;

use crate::dto::{NewPasswordDto, NewUserDto, SetupBodyDto, SuperuserDto};
use crate::error::ValidationSnafu;
use crate::services::password::hash_password;
use crate::{Result, run::AppState};

pub async fn setup_superuser_svc(state: &AppState, payload: SetupBodyDto) -> Result<SuperuserDto> {
    // Validate setup key
    ensure!(
        Some(payload.setup_key) == state.config.superuser.setup_key,
        ValidationSnafu {
            msg: "Invalid setup key".to_string(),
        }
    );

    // Make sure there are no superusers yet
    let superusers = state.db.superusers.list().await?;
    ensure!(
        superusers.is_empty(),
        ValidationSnafu {
            msg: "Superuser already exists".to_string(),
        }
    );

    let new_user = NewUserDto {
        email: payload.email,
        name: "Superuser".to_string(),
    };

    let new_password = NewPasswordDto {
        password: hash_password(&payload.password)?,
    };

    let superuser = state.db.superusers.setup(new_user, new_password).await?;

    Ok(superuser)
}

pub async fn setup_status_svc(state: &AppState) -> Result<bool> {
    let superusers = state.db.superusers.list().await?;
    Ok(!superusers.is_empty())
}
