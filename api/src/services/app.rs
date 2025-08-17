use serde::Deserialize;
use snafu::{ResultExt, ensure};
use validator::Validate;

use crate::Result;
use crate::error::{DbSnafu, ValidationSnafu};
use crate::state::AppState;
use db::app::{NewApp, UpdateApp};
use yaas::dto::AppDto;
use yaas::validators::flatten_errors;

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct NewAppDto {
    #[validate(length(min = 1, max = 100))]
    pub name: String,

    #[validate(length(min = 1, max = 200))]
    pub secret: String,

    #[validate(length(min = 1, max = 250))]
    #[validate(url)]
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdateAppDto {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,

    #[validate(length(min = 1, max = 200))]
    pub secret: Option<String>,

    #[validate(length(min = 1, max = 250))]
    #[validate(url)]
    pub redirect_uri: Option<String>,
}

pub async fn create_app(state: &AppState, data: &NewAppDto) -> Result<AppDto> {
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

pub async fn update_app(state: &AppState, id: &str, data: &UpdateAppDto) -> Result<bool> {
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

pub async fn delete_app(state: &AppState, id: &str) -> Result<bool> {
    state.db.apps.delete(id).await.context(DbSnafu)
}
