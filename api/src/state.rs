use axum::extract::FromRef;
use std::sync::Arc;

use crate::config::Config;
use db::DbMapper;

#[derive(Clone, FromRef)]
pub struct AppState {
    pub config: Config,
    pub db: Arc<DbMapper>,
}

#[cfg(test)]
pub fn create_test_app_state() -> AppState {
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

    AppState {
        config,
        db: Arc::new(db),
    }
}
