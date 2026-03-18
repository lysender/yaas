use snafu::ResultExt;

use crate::Result;
use crate::error::DbSnafu;
use crate::state::AppState;
use yaas::dto::{NewOauthCodeDto, OauthCodeDto};

pub async fn list_user_oauth_codes_svc(
    state: &AppState,
    user_id: &str,
) -> Result<Vec<OauthCodeDto>> {
    state
        .db
        .oauth_codes
        .list_by_user(user_id.to_string())
        .await
        .context(DbSnafu)
}

pub async fn create_oauth_code_svc(
    state: &AppState,
    data: NewOauthCodeDto,
) -> Result<OauthCodeDto> {
    state.db.oauth_codes.create(data).await.context(DbSnafu)
}

pub async fn delete_oauth_code_svc(state: &AppState, id: &str) -> Result<()> {
    state
        .db
        .oauth_codes
        .delete(id.to_string())
        .await
        .context(DbSnafu)
}
