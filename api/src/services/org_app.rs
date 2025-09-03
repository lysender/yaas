use db::org_app::NewOrgApp;
use snafu::{ResultExt, ensure};
use validator::Validate;

use crate::Result;
use crate::error::{DbSnafu, ValidationSnafu};
use crate::state::AppState;
use yaas::dto::{NewOrgAppDto, OrgAppDto};
use yaas::validators::flatten_errors;

pub async fn create_org_app(
    state: &AppState,
    org_id: i32,
    data: &NewOrgAppDto,
) -> Result<OrgAppDto> {
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let insert_data = NewOrgApp {
        org_id,
        app_id: data.app_id,
    };

    state
        .db
        .org_apps
        .create(&insert_data)
        .await
        .context(DbSnafu)
}

pub async fn delete_org_app(state: &AppState, id: i32) -> Result<()> {
    state.db.org_apps.delete(id).await.context(DbSnafu)
}
