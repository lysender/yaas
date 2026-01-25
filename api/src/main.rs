use snafu::ErrorCompat;
use std::process;
use tracing::info;

use crate::config::Config;
use run::run_server;

mod auth;
mod config;
mod error;
mod health;
mod run;
mod services;
mod state;
mod token;
mod web;

// Re-export error types for convenience
pub use error::{Error, Result};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    if dotenvy::dotenv().is_err() {
        info!("No .env file found, using existing environment variables instead.");
    }

    let config = Config::build().expect("Failed to build configuration");

    if let Err(e) = run_server(config).await {
        eprintln!("Application error: {}", e);
        if let Some(bt) = ErrorCompat::backtrace(&e) {
            println!("{}", bt);
        }
        process::exit(1);
    }
}
