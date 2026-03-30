use snafu::ResultExt;

use crate::Result;
use crate::error::DbSnafu;
use crate::state::AppState;
use yaas::dto::{AppDto, ListAppsParamsDto, NewAppDto, UpdateAppDto};
use yaas::pagination::Paginated;

pub async fn list_apps_svc(
    state: &AppState,
    params: ListAppsParamsDto,
) -> Result<Paginated<AppDto>> {
    state.db.apps.list(params).await.context(DbSnafu)
}

pub async fn create_app_svc(state: &AppState, data: NewAppDto) -> Result<AppDto> {
    state.db.apps.create(data).await.context(DbSnafu)
}

pub async fn get_app_svc(state: &AppState, id: &str) -> Result<Option<AppDto>> {
    state.db.apps.get(id.to_string()).await.context(DbSnafu)
}

pub async fn update_app_svc(state: &AppState, id: &str, data: UpdateAppDto) -> Result<bool> {
    state
        .db
        .apps
        .update(id.to_string(), data)
        .await
        .context(DbSnafu)
}

pub async fn regenerate_app_secret_svc(state: &AppState, id: &str) -> Result<bool> {
    state
        .db
        .apps
        .regenerate_secret(id.to_string())
        .await
        .context(DbSnafu)
}

pub async fn delete_app_svc(state: &AppState, id: &str) -> Result<bool> {
    state.db.apps.delete(id.to_string()).await.context(DbSnafu)
}
