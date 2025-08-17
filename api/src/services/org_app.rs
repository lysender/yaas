use db::org_app::NewOrgApp;
use serde::Deserialize;
use snafu::{ResultExt, ensure};
use validator::Validate;

use crate::Result;
use crate::error::{DbSnafu, ValidationSnafu};
use crate::state::AppState;
use yaas::dto::OrgAppDto;
use yaas::validators::flatten_errors;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct NewOrgAppDto {
    #[validate(length(equal = 36))]
    pub app_id: String,
}

pub async fn create_org_app(
    state: &AppState,
    org_id: &str,
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
        org_id: org_id.to_string(),
        app_id: data.app_id.clone(),
    };

    state
        .db
        .org_apps
        .create(&insert_data)
        .await
        .context(DbSnafu)
}

pub async fn delete_org_app(state: &AppState, id: &str) -> Result<()> {
    state.db.org_apps.delete(id).await.context(DbSnafu)
}
