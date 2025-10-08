use core::fmt;
use urlencoding::encode;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct UserParams {
    pub user_id: String,
}

#[derive(Deserialize)]
pub struct AppParams {
    pub app_id: String,
}

#[derive(Deserialize)]
pub struct OrgParams {
    pub org_id: String,
}
