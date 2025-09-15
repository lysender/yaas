use chrono::Utc;
use snafu::{ResultExt, ensure};
use validator::Validate;

use crate::Result;
use crate::error::{DbSnafu, ValidationSnafu};
use crate::state::AppState;
use db::org_member::{NewOrgMember, UpdateOrgMember};
use yaas::dto::{ListOrgMembersParamsDto, NewOrgMemberDto, OrgMemberDto, UpdateOrgMemberDto};
use yaas::pagination::Paginated;
use yaas::validators::flatten_errors;

pub async fn list_org_members_svc(
    state: &AppState,
    org_id: i32,
    params: ListOrgMembersParamsDto,
) -> Result<Paginated<OrgMemberDto>> {
    state
        .db
        .org_members
        .list(org_id, params)
        .await
        .context(DbSnafu)
}

pub async fn create_org_member_svc(
    state: &AppState,
    org_id: i32,
    data: NewOrgMemberDto,
) -> Result<OrgMemberDto> {
    // Ensure that the user exists
    let existing_user = state.db.users.get(data.user_id).await.context(DbSnafu)?;

    ensure!(
        existing_user.is_some(),
        ValidationSnafu {
            msg: "User does not exist".to_string(),
        }
    );

    // Ensure user is not already a member of the org
    let existing_member = state
        .db
        .org_members
        .find_member(org_id, data.user_id)
        .await
        .context(DbSnafu)?;

    ensure!(
        existing_member.is_none(),
        ValidationSnafu {
            msg: "User is already a member of the organization".to_string(),
        }
    );

    state
        .db
        .org_members
        .create(org_id, data)
        .await
        .context(DbSnafu)
}

pub async fn update_org_member_svc(
    state: &AppState,
    id: i32,
    data: UpdateOrgMemberDto,
) -> Result<bool> {
    state.db.org_members.update(id, data).await.context(DbSnafu)
}

pub async fn delete_org_member_svc(state: &AppState, id: i32) -> Result<()> {
    state.db.org_members.delete(id).await.context(DbSnafu)
}
