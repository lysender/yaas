use snafu::{ResultExt, ensure};

use crate::Result;
use crate::error::{DbSnafu, ValidationSnafu};
use crate::state::AppState;
use yaas::dto::{ListOrgAppsParamsDto, NewOrgAppDto, OrgAppDto, OrgAppSuggestionDto};
use yaas::pagination::Paginated;

pub async fn list_org_apps_svc(
    state: &AppState,
    org_id: i32,
    params: ListOrgAppsParamsDto,
) -> Result<Paginated<OrgAppDto>> {
    state
        .db
        .org_apps
        .list(org_id, params)
        .await
        .context(DbSnafu)
}

pub async fn list_org_app_suggestions_svc(
    state: &AppState,
    org_id: i32,
    params: ListOrgAppsParamsDto,
) -> Result<Paginated<OrgAppSuggestionDto>> {
    state
        .db
        .org_apps
        .list_app_suggestions(org_id, params)
        .await
        .context(DbSnafu)
}

pub async fn create_org_app_svc(
    state: &AppState,
    org_id: i32,
    data: NewOrgAppDto,
) -> Result<OrgAppDto> {
    // Ensure that the app exists
    let existing_app = state.db.apps.get(data.app_id).await.context(DbSnafu)?;

    ensure!(
        existing_app.is_some(),
        ValidationSnafu {
            msg: "App does not exist".to_string(),
        }
    );

    // Ensure that the app is not already linked to the org
    let existing_org_app = state
        .db
        .org_apps
        .find_app(org_id, data.app_id)
        .await
        .context(DbSnafu)?;

    ensure!(
        existing_org_app.is_none(),
        ValidationSnafu {
            msg: "App is already linked to the organization".to_string(),
        }
    );

    state
        .db
        .org_apps
        .create(org_id, data)
        .await
        .context(DbSnafu)
}

pub async fn get_org_app_svc(
    state: &AppState,
    org_id: i32,
    app_id: i32,
) -> Result<Option<OrgAppDto>> {
    state
        .db
        .org_apps
        .find_app(org_id, app_id)
        .await
        .context(DbSnafu)
}

pub async fn delete_org_app_svc(state: &AppState, id: i32) -> Result<()> {
    state.db.org_apps.delete(id).await.context(DbSnafu)
}
