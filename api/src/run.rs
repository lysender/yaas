use moka::sync::Cache;
use snafu::ResultExt;
use std::sync::Arc;
use std::time::Duration;
use tracing::info;

use db::{DbMapper, create_db_mapper};
use yaas::utils::generate_id;

use crate::{
    Result,
    config::{Config, SuperuserConfig},
    error::DbSnafu,
    state::AppState,
    web::server::run_web_server,
};

pub async fn run_server(config: Config) -> Result<()> {
    let db = Arc::new(create_db_mapper(&config.db.url));

    // 30 mins expiration with a small max capacity
    // We expect a light usage so this should be fine
    let auth_cache = Cache::builder()
        .time_to_live(Duration::from_secs(30 * 60))
        .time_to_idle(Duration::from_secs(5 * 60))
        .max_capacity(1000)
        .build();

    // Check for superusers
    let config = init_superuser(config, db.clone()).await?;

    let state = AppState {
        config,
        db,
        auth_cache,
    };

    run_web_server(state).await?;

    Ok(())
}

async fn init_superuser(mut config: Config, db: Arc<DbMapper>) -> Result<Config> {
    let superusers = db.superusers.list().await.context(DbSnafu)?;
    if superusers.is_empty() {
        let setup_key = generate_id("sup");
        info!("Superuser setup key: {}", setup_key);

        config.superuser = SuperuserConfig {
            setup_key: Some(setup_key),
        };
    }

    Ok(config)
}
