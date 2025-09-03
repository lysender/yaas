use snafu::{ResultExt, ensure};
use validator::Validate;

use crate::Result;
use crate::error::{DbSnafu, ValidationSnafu};
use crate::state::AppState;
use db::org::{NewOrg, UpdateOrg};
use yaas::dto::{NewOrgDto, OrgDto, UpdateOrgDto};
use yaas::validators::flatten_errors;

pub async fn create_org(state: &AppState, data: &NewOrgDto) -> Result<OrgDto> {
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let insert_data = NewOrg {
        name: data.name.clone(),
        owner_id: data.owner_id,
    };

    state.db.orgs.create(&insert_data).await.context(DbSnafu)
}

pub async fn update_org(state: &AppState, id: i32, data: &UpdateOrgDto) -> Result<bool> {
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    if data.name.is_none() || data.status.is_none() {
        return Ok(false);
    }

    let update_data = UpdateOrg {
        name: data.name.clone(),
        status: data.status.clone(),
        updated_at: Some(chrono::Utc::now()),
    };

    state
        .db
        .orgs
        .update(id, &update_data)
        .await
        .context(DbSnafu)
}

pub async fn delete_org(state: &AppState, id: i32) -> Result<bool> {
    state.db.orgs.delete(id).await.context(DbSnafu)
}
