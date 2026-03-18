use serde::Deserialize;
use snafu::ensure;
use std::env;

use crate::Result;
use crate::error::ConfigSnafu;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub jwt_secret: String,
    pub server: ServerConfig,
    pub db: DbConfig,
    pub superuser: SuperuserConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub port: u16,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DbConfig {
    pub filename: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SuperuserConfig {
    /// Key used to set up the superuser account
    pub setup_key: Option<String>,
}

impl Config {
    pub fn build() -> Result<Self> {
        // Build the config from ENV vars
        let jwt_secret = env::var("JWT_SECRET").expect("JWT_SECRET is required");
        let port = env::var("PORT")
            .expect("PORT is required")
            .parse::<u16>()
            .expect("PORT must be a valid u16");
        let db_file = env::var("DATABASE_FILE").expect("DATABASE_FILE is required");

        // Validate config values
        ensure!(
            !jwt_secret.is_empty(),
            ConfigSnafu {
                msg: "Jwt secret is required.".to_string()
            }
        );

        ensure!(
            !db_file.is_empty(),
            ConfigSnafu {
                msg: "Database file is required.".to_string()
            }
        );

        ensure!(
            port > 0,
            ConfigSnafu {
                msg: "Server port is required.".to_string()
            }
        );

        Ok(Config {
            jwt_secret,
            server: ServerConfig { port },
            db: DbConfig { filename: db_file },
            superuser: SuperuserConfig { setup_key: None },
        })
    }
}
