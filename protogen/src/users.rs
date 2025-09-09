use prost::Message;
use reqwest::{Client, StatusCode};
use tracing::info;

use crate::config::Config;
use yaas::buffed::dto::{ErrorMessageBuf, PaginatedUsersBuf};

pub async fn run_tests(client: &Client, config: &Config, token: &str) {
    info!("Running users tests");

    test_users_listing(client, config, token).await;
    test_users_listing_unauthenticated(client, config).await;
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
