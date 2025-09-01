use snafu::ResultExt;

use crate::Result;
use crate::error::DbSnafu;
use crate::state::AppState;
use yaas::dto::SuperuserDto;

pub async fn create_superuser(state: &AppState, user_id: i32) -> Result<SuperuserDto> {
    state.db.superusers.create(user_id).await.context(DbSnafu)
}

pub async fn get_superuser(state: &AppState, user_id: i32) -> Result<Option<SuperuserDto>> {
    state.db.superusers.get(user_id).await.context(DbSnafu)
}

pub async fn delete_org(state: &AppState, id: i32) -> Result<bool> {
    state.db.orgs.delete(id).await.context(DbSnafu)
}
