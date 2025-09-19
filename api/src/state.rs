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

#[cfg(test)]
pub fn create_test_app_state() -> AppState {
    use std::time::Duration;

    use crate::config::{DbConfig, ServerConfig, SuperuserConfig};
    use db::create_test_db_mapper;

    let config = Config {
        jwt_secret: "0196d1dbbfd87819b9183f14ac3ed485".to_string(),
        server: ServerConfig { port: 43700 },
        db: DbConfig {
            url: "url".to_string(),
        },
        superuser: SuperuserConfig { setup_key: None },
    };

    let db = create_test_db_mapper();

    let auth_cache = Cache::builder()
        .time_to_live(Duration::from_secs(30 * 60))
        .time_to_idle(Duration::from_secs(5 * 60))
        .max_capacity(100)
        .build();

    AppState {
        config,
        db: Arc::new(db),
        auth_cache,
    }
}
