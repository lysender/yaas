mod apps;
mod auth;
mod config;
mod org_apps;
mod org_members;
mod orgs;
mod smoke;
mod user;
mod users;

use prost::Message;
use reqwest::{Client, ClientBuilder, StatusCode};
use std::time::Duration;
use tracing::info;

use yaas::buffed::actor::{AuthResponseBuf, CredentialsBuf};
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

    let token = authenticate_superuser(&client, &config).await;

    smoke::run_tests(&client, &config).await;
    auth::run_tests(&client, &config).await;
    user::run_tests(&client, &config, &token).await;
    users::run_tests(&client, &config, &token).await;
    orgs::run_tests(&client, &config, &token).await;
    apps::run_tests(&client, &config, &token).await;
    org_members::run_tests(&client, &config, &token).await;
    org_apps::run_tests(&client, &config, &token).await;

    println!("Done");
}

async fn authenticate_superuser(client: &Client, config: &Config) -> String {
    info!("Authenticating superuser");

    let url = format!("{}/auth/authorize", &config.base_url);
    let body = CredentialsBuf {
        email: config.superuser_email.clone(),
        password: config.superuser_password.clone(),
    };

    let response = client
        .post(url)
        .body(prost::Message::encode_to_vec(&body))
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Response should be 200 OK"
    );

    let body_bytes = response
        .bytes()
        .await
        .expect("Should be able to read response body");

    let auth_response =
        AuthResponseBuf::decode(&body_bytes[..]).expect("Should be able to decode AuthResponseBuf");

    assert!(
        auth_response.user.is_some(),
        "AuthResponse should contain a user"
    );

    assert!(
        auth_response.token.is_some(),
        "AuthResponse should contain a token"
    );

    auth_response.token.unwrap()
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
        setup_key: "sup_01993bf2a969773294859be576cd6c61".to_string(),
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
