use serde::{Deserialize, Serialize};

use crate::buffed::dto::AppBuf;

#[derive(Clone, Serialize, Deserialize)]
pub struct AppDto {
    pub id: i32,
    pub name: String,
    pub secret: String,
    pub redirect_uri: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<AppBuf> for AppDto {
    fn from(app: AppBuf) -> Self {
        AppDto {
            id: app.id,
            name: app.name,
            secret: app.secret,
            redirect_uri: app.redirect_uri,
            created_at: app.created_at,
            updated_at: app.updated_at,
        }
    }
}
