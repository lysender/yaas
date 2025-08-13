use clap::Parser;
use snafu::ErrorCompat;
use std::process;
use tracing::info;

use crate::config::Config;

mod auth;
mod command;
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

    if let Err(_) = dotenvy::dotenv() {
        info!("No .env file found, using existing environment variables instead.");
    }

    let config = Config::build().expect("Failed to build configuration");

    println!("Hello, world!");
}
