use serde::Deserialize;
use std::env;

use crate::{Error, Result};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub jwt_secret: String,
    pub server: ServerConfig,
    pub db: DbConfig,
    pub superuser: SuperuserConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub address: String,
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
        Ok(Self {
            jwt_secret: required_env("JWT_SECRET")?,
            server: ServerConfig {
                address: required_env("SERVER_ADDRESS")?,
            },
            db: DbConfig {
                filename: required_env("DATABASE_FILE")?,
            },
            superuser: SuperuserConfig {
                setup_key: env::var("SUPERUSER_SETUP_KEY").ok(),
            },
        })
    }
}

fn required_env(name: &str) -> Result<String> {
    match env::var(name) {
        Ok(val) => {
            if val.is_empty() {
                return Err(Error::Config {
                    msg: format!("{} is required.", name),
                });
            }
            Ok(val)
        }
        Err(_) => Err(Error::Config {
            msg: format!("{} is required.", name),
        }),
    }
}
