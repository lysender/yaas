use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::buffed::dto::AppBuf;

#[derive(Clone, Serialize, Deserialize)]
pub struct AppDto {
    pub id: i32,
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<AppBuf> for AppDto {
    fn from(app: AppBuf) -> Self {
        AppDto {
            id: app.id,
            name: app.name,
            client_id: app.client_id,
            client_secret: app.client_secret,
            redirect_uri: app.redirect_uri,
            created_at: app.created_at,
            updated_at: app.updated_at,
        }
    }
}

#[derive(Clone, Deserialize, Validate)]
pub struct NewAppDto {
    #[validate(length(min = 1, max = 100))]
    pub name: String,

    #[validate(length(min = 1, max = 250))]
    #[validate(url)]
    pub redirect_uri: String,
}

#[derive(Clone, Deserialize, Validate)]
pub struct UpdateAppDto {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,

    #[validate(length(min = 1, max = 250))]
    #[validate(url)]
    pub redirect_uri: Option<String>,
}

#[derive(Clone, Deserialize, Validate)]
pub struct ListAppsParamsDto {
    #[validate(range(min = 1, max = 1000))]
    pub page: Option<i32>,

    #[validate(range(min = 1, max = 50))]
    pub per_page: Option<i32>,

    #[validate(length(min = 0, max = 50))]
    pub keyword: Option<String>,
}
