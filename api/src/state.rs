use axum::extract::FromRef;
use std::sync::Arc;

use crate::config::Config;
use db::DbMapper;

#[derive(Clone, FromRef)]
pub struct AppState {
    pub config: Config,
    pub db: Arc<DbMapper>,
}
