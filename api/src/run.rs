use snafu::ResultExt;
use std::sync::Arc;
use tracing::info;

use db::db::{DbMapper, create_db_mapper};
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

    // Check for superusers
    let config = init_superuser(config, db.clone()).await?;

    let state = AppState { config, db };

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
