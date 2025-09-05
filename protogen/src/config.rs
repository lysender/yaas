use serde::Deserialize;
use std::env;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub base_url: String,
    pub superuser_email: String,
    pub superuser_password: String,
}

impl Config {
    pub fn build() -> Self {
        // Build the config from ENV vars
        let base_url = env::var("BASE_URL").expect("BASE_URL is required");
        let superuser_email = env::var("SUPERUSER_EMAIL").expect("SUPERUSER_EMAIL is required");
        let superuser_password =
            env::var("SUPERUSER_PASSWORD").expect("SUPERUSER_PASSWORD is required");

        Config {
            base_url,
            superuser_email,
            superuser_password,
        }
    }
}
