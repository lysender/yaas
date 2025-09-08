use prost::Message;
use reqwest::{Client, StatusCode};
use tracing::info;

use yaas::buffed::actor::{AuthResponseBuf, CredentialsBuf};
use yaas::buffed::dto::{ErrorMessageBuf, PaginatedUsersBuf};

use crate::config::Config;

pub async fn run_tests(client: &Client, config: &Config) {
    info!("Running users tests");

    let superuser_token = authenticate_superuser(client, config).await;

    test_users_listing(client, config, &superuser_token).await;
    test_users_listing_unauthenticated(client, config).await;
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

async fn test_users_listing(client: &Client, config: &Config, token: &str) {
    info!("test_user_profile");

    let url = format!("{}/users", &config.base_url);
    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {}", token))
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

    let listing = PaginatedUsersBuf::decode(&body_bytes[..])
        .expect("Should be able to decode PaginatedUsersBuf");

    let meta = listing.meta.unwrap();
    assert!(meta.page == 1, "Page should be 1");
    assert!(meta.per_page == 50, "Per page should be 50");
    assert!(meta.total_records >= 1, "Total records should be >= 1");
    assert!(meta.total_pages >= 1, "Total pages should be >= 1");

    assert!(listing.data.len() >= 1, "There should be at least one user");
}

async fn test_users_listing_unauthenticated(client: &Client, config: &Config) {
    info!("test_user_profile_unauthenticated");

    let url = format!("{}/users", &config.base_url);
    let response = client
        .get(url)
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Response should be 401 Unauthorized"
    );

    let body_bytes = response
        .bytes()
        .await
        .expect("Should be able to read response body");

    let error_message =
        ErrorMessageBuf::decode(&body_bytes[..]).expect("Should be able to decode ErrorMessageBuf");

    assert_eq!(
        error_message.status_code, 401,
        "Error status code should be 401 Unauthorized"
    );
}
