use serde::{Deserialize, Serialize};
use snafu::ensure;

use crate::dto::Paginated;
use crate::dto::{AppDto, ListAppsParamsDto, NewAppDto, UpdateAppDto};
use crate::error::CsrfTokenSnafu;
use crate::run::AppState;
use crate::services::token::verify_csrf_token;
use crate::{Error, Result};

#[derive(Clone, Deserialize, Serialize)]
pub struct NewAppFormData {
    pub name: String,
    pub redirect_uri: String,
    pub token: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UpdateAppFormData {
    pub token: String,
    pub name: String,
    pub redirect_uri: String,
}

pub async fn list_apps_svc(
    state: &AppState,
    params: ListAppsParamsDto,
) -> Result<Paginated<AppDto>> {
    state.db.apps.list(params).await
}

pub async fn create_app_svc(state: &AppState, data: NewAppDto) -> Result<AppDto> {
    state.db.apps.create(data).await
}

pub async fn create_app_web_svc(state: &AppState, form: NewAppFormData) -> Result<AppDto> {
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == "new_app", CsrfTokenSnafu);

    create_app_svc(
        state,
        NewAppDto {
            name: form.name,
            redirect_uri: form.redirect_uri,
        },
    )
    .await
}

pub async fn get_app_svc(state: &AppState, id: &str) -> Result<Option<AppDto>> {
    state.db.apps.get(id.to_string()).await
}

pub async fn update_app_svc(state: &AppState, id: &str, data: UpdateAppDto) -> Result<bool> {
    state.db.apps.update(id.to_string(), data).await
}

pub async fn update_app_web_svc(
    state: &AppState,
    app_id: &str,
    form: UpdateAppFormData,
) -> Result<AppDto> {
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == app_id, CsrfTokenSnafu);

    update_app_svc(
        state,
        app_id,
        UpdateAppDto {
            name: Some(form.name),
            redirect_uri: Some(form.redirect_uri),
        },
    )
    .await?;

    // Fetch the updated app to return
    let Some(updated_app) = get_app_svc(state, app_id).await? else {
        return Err(Error::AppNotFound);
    };

    Ok(updated_app)
}

pub async fn regenerate_app_secret_svc(state: &AppState, id: &str) -> Result<bool> {
    state.db.apps.regenerate_secret(id.to_string()).await
}

pub async fn regenerate_app_secret_web_svc(
    state: &AppState,
    app_id: &str,
    csrf_token: &str,
) -> Result<()> {
    let csrf_result = verify_csrf_token(csrf_token, &state.config.jwt_secret)?;
    ensure!(csrf_result == app_id, CsrfTokenSnafu);

    regenerate_app_secret_svc(state, app_id).await?;

    Ok(())
}

pub async fn delete_app_svc(state: &AppState, id: &str) -> Result<bool> {
    state.db.apps.delete(id.to_string()).await
}

pub async fn delete_app_web_svc(state: &AppState, app_id: &str, csrf_token: &str) -> Result<()> {
    let csrf_result = verify_csrf_token(csrf_token, &state.config.jwt_secret)?;
    ensure!(csrf_result == app_id, CsrfTokenSnafu);

    delete_app_svc(state, app_id).await?;

    Ok(())
}
