use password::hash_password;
use snafu::{ResultExt, ensure};

use crate::error::{DbSnafu, PasswordSnafu};
use crate::state::AppState;
use crate::{Result, error::ValidationSnafu};
use yaas::dto::{NewPasswordDto, NewUserDto, SetupBodyDto, SuperuserDto};

pub async fn setup_superuser_svc(state: &AppState, payload: SetupBodyDto) -> Result<SuperuserDto> {
    // Validate setup key
    ensure!(
        Some(payload.setup_key) == state.config.superuser.setup_key,
        ValidationSnafu {
            msg: "Invalid setup key".to_string(),
        }
    );

    // Make sure there are no superusers yet
    let superusers = state.db.superusers.list().await.context(DbSnafu)?;
    ensure!(
        superusers.len() == 0,
        ValidationSnafu {
            msg: "Superuser already exists".to_string(),
        }
    );

    let new_user = NewUserDto {
        email: payload.email,
        name: "Superuser".to_string(),
    };

    let new_password = NewPasswordDto {
        password: hash_password(&payload.password).context(PasswordSnafu)?,
    };

    let superuser = state
        .db
        .superusers
        .setup(new_user, new_password)
        .await
        .context(DbSnafu)?;

    Ok(superuser)
}

async fn create_superuser_svc(state: &AppState, user_id: i32) -> Result<SuperuserDto> {
    state.db.superusers.create(user_id).await.context(DbSnafu)
}

pub async fn get_superuser_svc(state: &AppState, user_id: i32) -> Result<Option<SuperuserDto>> {
    state.db.superusers.get(user_id).await.context(DbSnafu)
}
