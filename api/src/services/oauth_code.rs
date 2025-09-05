use snafu::{ResultExt, ensure};
use validator::Validate;

use crate::Result;
use crate::error::{DbSnafu, ValidationSnafu};
use crate::state::AppState;
use db::oauth_code::NewOauthCode;
use yaas::dto::{NewOauthCodeDto, OauthCodeDto};
use yaas::validators::flatten_errors;

pub async fn create_oauth_code_svc(
    state: &AppState,
    data: &NewOauthCodeDto,
) -> Result<OauthCodeDto> {
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

pub async fn delete_oauth_code_svc(state: &AppState, id: i32) -> Result<()> {
    state.db.oauth_codes.delete(id).await.context(DbSnafu)
}
