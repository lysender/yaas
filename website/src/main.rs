mod config;
mod ctx;
mod error;
mod models;
mod run;
mod services;
mod web;

use std::process;

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

    let config = Config::build();
    if let Err(e) = run(config).await {
        eprintln!("Application error: {e}");
        process::exit(1);
    }
}
