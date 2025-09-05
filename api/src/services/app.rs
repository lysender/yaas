use snafu::{ResultExt, ensure};
use validator::Validate;

use crate::Result;
use crate::error::{DbSnafu, ValidationSnafu};
use crate::state::AppState;
use db::app::{NewApp, UpdateApp};
use yaas::dto::{AppDto, NewAppDto, UpdateAppDto};
use yaas::validators::flatten_errors;

pub async fn create_app_svc(state: &AppState, data: &NewAppDto) -> Result<AppDto> {
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let insert_data = NewApp {
        name: data.name.clone(),
        secret: data.secret.clone(),
        redirect_uri: data.redirect_uri.clone(),
    };

    state.db.apps.create(&insert_data).await.context(DbSnafu)
}

pub async fn update_app_svc(state: &AppState, id: i32, data: &UpdateAppDto) -> Result<bool> {
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    if data.name.is_none() || data.secret.is_none() || data.redirect_uri.is_none() {
        return Ok(false);
    }

    let update_data = UpdateApp {
        name: data.name.clone(),
        secret: data.secret.clone(),
        redirect_uri: data.redirect_uri.clone(),
        updated_at: Some(chrono::Utc::now()),
    };

    state
        .db
        .apps
        .update(id, &update_data)
        .await
        .context(DbSnafu)
}

pub async fn delete_app_svc(state: &AppState, id: i32) -> Result<bool> {
    state.db.apps.delete(id).await.context(DbSnafu)
}
