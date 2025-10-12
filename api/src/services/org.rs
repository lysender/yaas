use snafu::{ResultExt, ensure};

use crate::Result;
use crate::error::{DbSnafu, ValidationSnafu};
use crate::state::AppState;
use yaas::dto::{ListOrgsParamsDto, NewOrgDto, OrgDto, UpdateOrgDto};
use yaas::pagination::Paginated;

pub async fn list_orgs_svc(
    state: &AppState,
    params: ListOrgsParamsDto,
) -> Result<Paginated<OrgDto>> {
    state.db.orgs.list(params).await.context(DbSnafu)
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

    state.db.orgs.create(data).await.context(DbSnafu)
}

pub async fn get_org_svc(state: &AppState, id: i32) -> Result<Option<OrgDto>> {
    state.db.orgs.get(id).await.context(DbSnafu)
}

pub async fn update_org_svc(state: &AppState, id: i32, data: UpdateOrgDto) -> Result<bool> {
    // Owner must exists and must be a member of the org
    if let Some(owner_id) = data.owner_id {
        let owner = state.db.users.get(owner_id).await.context(DbSnafu)?;
        ensure!(
            owner.is_some(),
            ValidationSnafu {
                msg: "Owner does not exists".to_string()
            }
        );

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
    }

    state.db.orgs.update(id, data).await.context(DbSnafu)
}

pub async fn delete_org_svc(state: &AppState, id: i32) -> Result<bool> {
    state.db.orgs.delete(id).await.context(DbSnafu)
}
