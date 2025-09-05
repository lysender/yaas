mod auth;
mod config;
mod smoke;
mod user;

use reqwest::ClientBuilder;
use std::time::Duration;
use tracing::info;

use yaas::buffed::actor::CredentialsBuf;
use yaas::buffed::dto::{ChangeCurrentPasswordBuf, SetupBodyBuf};

use crate::config::Config;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(false)
        .compact()
        .init();

    if let Err(_) = dotenvy::dotenv() {
        info!("No .env file found, using existing environment variables instead.");
    }

    let config = Config::build();

    write_credentials();
    write_setup_payload();
    write_change_password_payload();

    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(3))
        .build()
        .expect("HTTP Client is required");

    smoke::run_tests(&client, &config).await;
    auth::run_tests(&client, &config).await;
    user::run_tests(&client, &config).await;

    println!("Done");
}

fn write_change_password_payload() {
    let body = ChangeCurrentPasswordBuf {
        current_password: "password123".to_string(),
        new_password: "password".to_string(),
    };

    let filename = "buffs/change_password.buf";
    let bytes = prost::Message::encode_to_vec(&body);

    std::fs::write(filename, &bytes).expect("Unable to write file");
}

fn write_setup_payload() {
    let body = SetupBodyBuf {
        setup_key: "sup_01991aa39bb878e0ac56bc0baedc701f".to_string(),
        email: "root@example.com".to_string(),
        password: "password".to_string(),
    };

    let filename = "buffs/setup.buf";
    let bytes = prost::Message::encode_to_vec(&body);

    std::fs::write(filename, &bytes).expect("Unable to write file");
}

fn write_credentials() {
    let credentials = CredentialsBuf {
        email: "root@example.com".to_string(),
        password: "password".to_string(),
    };

    let filename = "buffs/credentials.buf";
    let bytes = prost::Message::encode_to_vec(&credentials);

    std::fs::write(filename, &bytes).expect("Unable to write file");
}
