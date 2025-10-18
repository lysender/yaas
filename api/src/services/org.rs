use snafu::{ResultExt, ensure};

use crate::Result;
use crate::error::{DbSnafu, ValidationSnafu};
use crate::state::AppState;
use yaas::dto::{
    ListOrgOwnerSuggestionsParamsDto, ListOrgsParamsDto, NewOrgDto, OrgDto, OrgOwnerSuggestionDto,
    UpdateOrgDto,
};
use yaas::pagination::Paginated;

pub async fn list_orgs_svc(
    state: &AppState,
    params: ListOrgsParamsDto,
) -> Result<Paginated<OrgDto>> {
    state.db.orgs.list(params).await.context(DbSnafu)
}

pub async fn list_org_owner_suggestions_svc(
    state: &AppState,
    params: ListOrgOwnerSuggestionsParamsDto,
) -> Result<Paginated<OrgOwnerSuggestionDto>> {
    state
        .db
        .orgs
        .list_owner_suggestions(params)
        .await
        .context(DbSnafu)
}

pub async fn create_org_svc(state: &AppState, data: NewOrgDto) -> Result<OrgDto> {
    // Owner must exists
    let owner = state.db.users.get(data.owner_id).await.context(DbSnafu)?;
    ensure!(
        owner.is_some(),
        ValidationSnafu {
            msg: "Owner does not exists".to_string()
        }
    );

    // Owner must not be a superuser
    let superuser = state
        .db
        .superusers
        .get(data.owner_id)
        .await
        .context(DbSnafu)?;

    ensure!(
        superuser.is_none(),
        ValidationSnafu {
            msg: "Owner cannot be a superuser".to_string()
        }
    );

    state.db.orgs.create(data).await.context(DbSnafu)
}

pub async fn get_org_svc(state: &AppState, id: i32) -> Result<Option<OrgDto>> {
    state.db.orgs.get(id).await.context(DbSnafu)
}

pub async fn update_org_svc(state: &AppState, id: i32, data: UpdateOrgDto) -> Result<bool> {
    // Owner must exists and must be a member of the org
    if let Some(owner_id) = data.owner_id {
        // User must exists
        let owner = state.db.users.get(owner_id).await.context(DbSnafu)?;
        ensure!(
            owner.is_some(),
            ValidationSnafu {
                msg: "Owner does not exists".to_string()
            }
        );

        // Owner must be an existing member of the org
        let member = state
            .db
            .org_members
            .find_member(id, owner_id)
            .await
            .context(DbSnafu)?;

        ensure!(
            member.is_some(),
            ValidationSnafu {
                msg: "Owner must be a member of the org".to_string()
            }
        );

        // Owner must not be a superuser
        let superuser = state.db.superusers.get(owner_id).await.context(DbSnafu)?;

        ensure!(
            superuser.is_none(),
            ValidationSnafu {
                msg: "Owner cannot be a superuser".to_string()
            }
        );
    }

    state.db.orgs.update(id, data).await.context(DbSnafu)
}

pub async fn delete_org_svc(state: &AppState, id: i32) -> Result<bool> {
    // TODO: Check if org has members or apps before deleting
    state.db.orgs.delete(id).await.context(DbSnafu)
}
