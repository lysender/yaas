use axum::extract::FromRef;
use moka::sync::Cache;
use std::sync::Arc;
use yaas::dto::UserDto;

use crate::config::Config;
use db::DbMapper;

#[derive(Clone, FromRef)]
pub struct AppState {
    pub config: Config,
    pub db: Arc<DbMapper>,
    pub auth_cache: Cache<i32, UserDto>,
}
