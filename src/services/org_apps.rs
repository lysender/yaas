use serde::{Deserialize, Serialize};
use snafu::ensure;

use crate::Result;
use crate::ctx::Ctx;
use crate::dto::Paginated;
use crate::dto::{ListOrgAppsParamsDto, NewOrgAppDto, OrgAppDto, OrgAppSuggestionDto};
use crate::error::{CsrfTokenSnafu, ValidationSnafu};
use crate::run::AppState;
use crate::services::token::verify_csrf_token;

#[derive(Clone, Deserialize, Serialize)]
pub struct NewOrgAppFormData {
    pub token: String,
    pub app_id: String,
    pub app_name: String,
}

pub async fn list_org_apps_svc(
    state: &AppState,
    org_id: &str,
    params: ListOrgAppsParamsDto,
) -> Result<Paginated<OrgAppDto>> {
    state.db.org_apps.list(org_id.to_string(), params).await
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
}

pub async fn create_org_app_svc(
    state: &AppState,
    org_id: &str,
    data: NewOrgAppDto,
) -> Result<OrgAppDto> {
    // Ensure that the app exists
    let app_id = data.app_id.clone();
    let existing_app = state.db.apps.get(app_id.clone()).await?;

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
        .await?;

    ensure!(
        existing_org_app.is_none(),
        ValidationSnafu {
            msg: "App is already linked to the organization".to_string(),
        }
    );

    state.db.org_apps.create(org_id.to_string(), data).await
}

pub async fn create_org_app_web_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: &str,
    form: NewOrgAppFormData,
) -> Result<OrgAppDto> {
    let token = ctx.token().expect("Token is required");
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == "new_org_app", CsrfTokenSnafu);

    create_org_app_svc(
        state,
        org_id,
        NewOrgAppDto {
            app_id: form.app_id,
        },
    )
    .await
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
}

pub async fn delete_org_app_svc(state: &AppState, id: &str) -> Result<()> {
    state.db.org_apps.delete(id.to_string()).await
}

pub async fn delete_org_app_web_svc(
    state: &AppState,
    org_id: &str,
    app_id: &str,
    csrf_token: &str,
) -> Result<()> {
    let csrf_result = verify_csrf_token(csrf_token, &state.config.jwt_secret)?;
    ensure!(csrf_result == app_id, CsrfTokenSnafu);

    // Ensure the org_app actually belongs to the org
    let existing_org_app = get_org_app_svc(state, org_id, app_id).await?;

    ensure!(
        existing_org_app.is_some(),
        ValidationSnafu {
            msg: "App is not linked to the organization".to_string(),
        }
    );

    delete_org_app_svc(state, app_id).await?;

    Ok(())
}
