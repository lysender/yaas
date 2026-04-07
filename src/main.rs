mod config;
mod ctx;
mod db;
mod dto;
mod error;
mod models;
mod run;
mod services;
#[cfg(test)]
mod test;
mod utils;
mod validators;
mod web;

use std::process;
use tracing::info;

use config::Config;

// Re-exports
pub use error::{Error, Result};

use crate::run::run;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    if dotenvy::dotenv().is_err() {
        info!("No .env file found, using existing environment variables instead.");
    }

    if let Err(e) = run_command().await {
        eprintln!("Application error: {e}");
        process::exit(1);
    }
}

async fn run_command() -> Result<()> {
    let config = Config::build()?;
    run(config).await
}
