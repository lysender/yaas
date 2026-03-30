use crate::Result;
use crate::dto::{NewOauthCodeDto, OauthCodeDto};
use crate::run::AppState;

pub async fn create_oauth_code_svc(
    state: &AppState,
    data: NewOauthCodeDto,
) -> Result<OauthCodeDto> {
    state.db.oauth_codes.create(data).await
}

pub async fn delete_oauth_code_svc(state: &AppState, id: &str) -> Result<()> {
    state.db.oauth_codes.delete(id.to_string()).await
}
