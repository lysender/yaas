use db::org::NewOrg;
use db::org_member::NewOrgMember;
use snafu::{ResultExt, ensure};
use yaas::role::Role;

use crate::error::DbSnafu;
use crate::services::password::{NewPasswordDto, create_password};
use crate::services::user::{NewUserDto, create_user};
use crate::state::AppState;
use crate::{Result, error::ValidationSnafu};
use yaas::xdto::{SetupBodyDto, SuperuserDto, UserDto};

pub async fn setup_superuser(state: &AppState, payload: SetupBodyDto) -> Result<UserDto> {
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

    // Create user
    let user = create_user(state, &new_user).await?;

    // Create password
    let new_password = NewPasswordDto {
        password: payload.password,
    };

    let _ = create_password(state, user.id, &new_password).await?;

    // Create organization
    let new_org = NewOrg {
        name: "Superuser".to_string(),
        owner_id: user.id,
    };
    let org = state.db.orgs.create(&new_org).await.context(DbSnafu)?;

    // Add as member
    let new_member = NewOrgMember {
        user_id: user.id,
        roles: vec![Role::Superuser.to_string()],
        status: "active".to_string(),
    };
    let _ = state
        .db
        .org_members
        .create(org.id, &new_member)
        .await
        .context(DbSnafu)?;

    // Create superuser entry
    let _ = create_superuser(state, user.id).await?;

    Ok(user)
}

async fn create_superuser(state: &AppState, user_id: i32) -> Result<SuperuserDto> {
    state.db.superusers.create(user_id).await.context(DbSnafu)
}

pub async fn get_superuser(state: &AppState, user_id: i32) -> Result<Option<SuperuserDto>> {
    state.db.superusers.get(user_id).await.context(DbSnafu)
}

pub async fn delete_org(state: &AppState, id: i32) -> Result<bool> {
    state.db.orgs.delete(id).await.context(DbSnafu)
}
