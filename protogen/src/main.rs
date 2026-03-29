mod apps;
mod auth;
mod config;
mod oauth;
mod org_apps;
mod org_members;
mod orgs;
mod smoke;
mod user;
mod users;

use reqwest::{Client, ClientBuilder, StatusCode};
use std::time::Duration;
use tracing::info;
use yaas::dto::{AuthResponseDto, CredentialsDto};

use crate::config::Config;

pub struct TestActor {
    pub id: String,
    pub email: String,
    pub token: String,
}

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

    write_credentials();
    write_other_credentials();
    write_setup_payload();
    write_change_password_payload();

    let client = ClientBuilder::new()
        .timeout(Duration::from_secs(3))
        .build()
        .expect("HTTP Client is required");

    let actor = authenticate_superuser(&client, &config).await;

    smoke::run_tests(&client, &config).await;
    auth::run_tests(&client, &config).await;
    user::run_tests(&client, &config, &actor).await;
    users::run_tests(&client, &config, &actor).await;
    orgs::run_tests(&client, &config, &actor).await;
    apps::run_tests(&client, &config, &actor).await;
    org_members::run_tests(&client, &config, &actor).await;
    org_apps::run_tests(&client, &config, &actor).await;
    oauth::run_tests(&client, &config, &actor).await;

    println!("Done");
}

async fn authenticate_superuser(client: &Client, config: &Config) -> TestActor {
    info!("Authenticating superuser");

    authenticate_user(
        client,
        config,
        CredentialsDto {
            email: config.superuser_email.clone(),
            password: config.superuser_password.clone(),
        },
    )
    .await
}

pub async fn authenticate_user(
    client: &Client,
    config: &Config,
    credentials: CredentialsDto,
) -> TestActor {
    info!("authenticate_user");

    let url = format!("{}/auth/authorize", &config.base_url);
    let body = serde_json::json!({
        "email": credentials.email,
        "password": credentials.password,
    });

    let response = client
        .post(url)
        .json(&body)
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Response should be 200 OK"
    );

    let auth_response = response
        .json::<AuthResponseDto>()
        .await
        .expect("Should be able to decode AuthResponseDto");

    assert!(
        !auth_response.token.is_empty(),
        "AuthResponse should contain a token"
    );

    let user = auth_response.user;
    let token = auth_response.token;

    TestActor {
        id: user.id,
        email: user.email,
        token,
    }
}

fn write_change_password_payload() {
    let body = serde_json::json!({
        "current_password": "password123",
        "new_password": "password",
    });

    let filename = "buffs/change_password.json";
    let bytes = serde_json::to_vec_pretty(&body).expect("Should serialize JSON payload");

    std::fs::write(filename, &bytes).expect("Unable to write file");
}

fn write_setup_payload() {
    let body = serde_json::json!({
        "setup_key": "suk_019d012c68dd75b2a9d409095301c205",
        "email": "root@example.com",
        "password": "password",
    });

    let filename = "buffs/setup.json";
    let bytes = serde_json::to_vec_pretty(&body).expect("Should serialize JSON payload");

    std::fs::write(filename, &bytes).expect("Unable to write file");
}

fn write_credentials() {
    let credentials = serde_json::json!({
        "email": "root@example.com",
        "password": "password",
    });

    let filename = "buffs/credentials.json";
    let bytes = serde_json::to_vec_pretty(&credentials).expect("Should serialize JSON payload");

    std::fs::write(filename, &bytes).expect("Unable to write file");
}

fn write_other_credentials() {
    let credentials = serde_json::json!({
        "email": "luffy@lysender.com",
        "password": "password",
    });

    let filename = "buffs/other_credentials.json";
    let bytes = serde_json::to_vec_pretty(&credentials).expect("Should serialize JSON payload");

    std::fs::write(filename, &bytes).expect("Unable to write file");
}
