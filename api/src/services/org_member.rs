use chrono::Utc;
use serde::Deserialize;
use snafu::{ResultExt, ensure};
use validator::Validate;

use crate::Result;
use crate::error::{DbSnafu, ValidationSnafu};
use crate::state::AppState;
use db::org_member::{NewOrgMember, UpdateOrgMember};
use yaas::validators::flatten_errors;
use yaas::xdto::OrgMemberDto;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct NewOrgMemberDto {
    pub user_id: i32,

    #[validate(length(min = 1, max = 20))]
    #[validate(custom(function = "yaas::validators::roles"))]
    pub roles: Vec<String>,

    #[validate(custom(function = "yaas::validators::status"))]
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateOrgMemberDto {
    #[validate(length(min = 1, max = 20))]
    #[validate(custom(function = "yaas::validators::roles"))]
    pub roles: Option<Vec<String>>,

    #[validate(custom(function = "yaas::validators::status"))]
    pub status: Option<String>,
}

pub async fn create_org_member(
    state: &AppState,
    org_id: i32,
    data: &NewOrgMemberDto,
) -> Result<OrgMemberDto> {
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    // Ensure that the user exists
    let existing_user = state.db.users.get(data.user_id).await.context(DbSnafu)?;

    ensure!(
        existing_user.is_some(),
        ValidationSnafu {
            msg: "User does not exist".to_string(),
        }
    );

    // TODO: Check if the user is already a member of the organization

    let insert_data = NewOrgMember {
        user_id: data.user_id.clone(),
        roles: data.roles.clone(),
        status: data.status.clone(),
    };

    state
        .db
        .org_members
        .create(org_id, &insert_data)
        .await
        .context(DbSnafu)
}

pub async fn update_org_member(
    state: &AppState,
    id: i32,
    data: &UpdateOrgMemberDto,
) -> Result<bool> {
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    if data.roles.is_none() || data.status.is_none() {
        return Ok(false);
    }

    let updated_at = Some(Utc::now());

    let update_data = UpdateOrgMember {
        roles: data.roles.as_ref().map(|x| x.join(",")),
        status: data.status.clone(),
        updated_at,
    };

    state
        .db
        .org_members
        .update(id, &update_data)
        .await
        .context(DbSnafu)
}

pub async fn delete_org_member(state: &AppState, id: i32) -> Result<()> {
    state.db.org_members.delete(id).await.context(DbSnafu)
}
