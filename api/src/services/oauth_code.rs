use serde::Deserialize;
use snafu::{ResultExt, ensure};
use validator::Validate;

use crate::Result;
use crate::error::{DbSnafu, ValidationSnafu};
use crate::state::AppState;
use db::oauth_code::NewOauthCode;
use yaas::validators::flatten_errors;
use yaas::xdto::OauthCodeDto;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct NewOauthCodeDto {
    #[validate(length(equal = 36))]
    pub code: String,

    #[validate(length(min = 1, max = 250))]
    pub state: String,

    #[validate(length(min = 1, max = 250))]
    #[validate(url)]
    pub redirect_uri: String,

    #[validate(length(min = 1, max = 250))]
    pub scope: String,

    pub app_id: i32,
    pub org_id: i32,
    pub user_id: i32,
}

pub async fn create_oauth_code(state: &AppState, data: &NewOauthCodeDto) -> Result<OauthCodeDto> {
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let insert_data = NewOauthCode {
        code: data.code.clone(),
        state: data.state.clone(),
        redirect_uri: data.redirect_uri.clone(),
        scope: data.scope.clone(),
        app_id: data.app_id,
        org_id: data.org_id,
        user_id: data.user_id,
    };

    state
        .db
        .oauth_codes
        .create(&insert_data)
        .await
        .context(DbSnafu)
}

pub async fn delete_oauth_code(state: &AppState, id: i32) -> Result<()> {
    state.db.oauth_codes.delete(id).await.context(DbSnafu)
}
