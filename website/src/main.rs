mod config;
mod ctx;
mod error;
mod models;
mod run;
mod services;
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

    let config = Config::build();
    if let Err(e) = run(config).await {
        eprintln!("Application error: {e}");
        process::exit(1);
    }
}
