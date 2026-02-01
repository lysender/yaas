use core::fmt;
use serde::{Deserialize, Serialize};
use urlencoding::encode;
use validator::Validate;

use crate::buffed::dto::{AppBuf, NewAppBuf, UpdateAppBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
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

impl From<NewAppBuf> for NewAppDto {
    fn from(app: NewAppBuf) -> Self {
        NewAppDto {
            name: app.name,
            redirect_uri: app.redirect_uri,
        }
    }
}

#[derive(Clone, Deserialize, Validate)]
pub struct UpdateAppDto {
    #[validate(length(min = 1, max = 100))]
    pub name: Option<String>,

    #[validate(length(min = 1, max = 250))]
    #[validate(url)]
    pub redirect_uri: Option<String>,
}

impl From<UpdateAppBuf> for UpdateAppDto {
    fn from(app: UpdateAppBuf) -> Self {
        UpdateAppDto {
            name: app.name,
            redirect_uri: app.redirect_uri,
        }
    }
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

impl Default for ListAppsParamsDto {
    fn default() -> Self {
        Self {
            keyword: None,
            page: Some(1),
            per_page: Some(10),
        }
    }
}

impl fmt::Display for ListAppsParamsDto {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // Ideally, we want an empty string if all fields are None
        if self.keyword.is_none() && self.page.is_none() && self.per_page.is_none() {
            return write!(f, "");
        }

        let keyword = self.keyword.as_deref().unwrap_or("");
        let page = self.page.unwrap_or(1);
        let per_page = self.per_page.unwrap_or(10);

        write!(
            f,
            "page={}&per_page={}&keyword={}",
            page,
            per_page,
            encode(keyword)
        )
    }
}
