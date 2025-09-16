use snafu::ResultExt;

use crate::Result;
use crate::error::DbSnafu;
use crate::state::AppState;
use yaas::dto::{NewOauthCodeDto, OauthCodeDto};

pub async fn create_oauth_code_svc(
    state: &AppState,
    data: NewOauthCodeDto,
) -> Result<OauthCodeDto> {
    state.db.oauth_codes.create(data).await.context(DbSnafu)
}

pub async fn delete_oauth_code_svc(state: &AppState, id: i32) -> Result<()> {
    state.db.oauth_codes.delete(id).await.context(DbSnafu)
}
