use snafu::{ResultExt, ensure};

use crate::Result;
use crate::error::{DbSnafu, ValidationSnafu};
use crate::state::AppState;
use yaas::dto::{ListOrgAppsParamsDto, NewOrgAppDto, OrgAppDto, OrgAppSuggestionDto};
use yaas::pagination::Paginated;

pub async fn list_org_apps_svc(
    state: &AppState,
    org_id: &str,
    params: ListOrgAppsParamsDto,
) -> Result<Paginated<OrgAppDto>> {
    state
        .db
        .org_apps
        .list(org_id.to_string(), params)
        .await
        .context(DbSnafu)
}

pub async fn list_org_app_suggestions_svc(
    state: &AppState,
    org_id: &str,
    params: ListOrgAppsParamsDto,
) -> Result<Paginated<OrgAppSuggestionDto>> {
    state
        .db
        .org_apps
        .list_app_suggestions(org_id.to_string(), params)
        .await
        .context(DbSnafu)
}

pub async fn create_org_app_svc(
    state: &AppState,
    org_id: &str,
    data: NewOrgAppDto,
) -> Result<OrgAppDto> {
    // Ensure that the app exists
    let app_id = data.app_id.clone();
    let existing_app = state.db.apps.get(app_id.clone()).await.context(DbSnafu)?;

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
        .find_app(org_id.to_string(), app_id)
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
        .create(org_id.to_string(), data)
        .await
        .context(DbSnafu)
}

pub async fn get_org_app_svc(
    state: &AppState,
    org_id: &str,
    app_id: &str,
) -> Result<Option<OrgAppDto>> {
    state
        .db
        .org_apps
        .find_app(org_id.to_string(), app_id.to_string())
        .await
        .context(DbSnafu)
}

pub async fn delete_org_app_svc(state: &AppState, id: &str) -> Result<()> {
    state
        .db
        .org_apps
        .delete(id.to_string())
        .await
        .context(DbSnafu)
}
