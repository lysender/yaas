use prost::Message;
use reqwest::{Client, StatusCode};
use tracing::info;

use yaas::buffed::dto::{ErrorMessageBuf, SetupBodyBuf};
use yaas::utils::generate_id;

use crate::config::Config;

pub async fn run_tests(client: &Client, config: &Config) {
    info!("Running smoke tests");

    test_home(client, config).await;
    test_not_found(client, config).await;
    test_setup(client, config).await;
    test_health_liveness(client, config).await;
    test_health_readiness(client, config).await;
}

async fn test_home(client: &Client, config: &Config) {
    info!("test_home");

    let response = client
        .get(&config.base_url)
        .send()
        .await
        .expect("Should be able to send request");

    // TODO: Body should be in protobuf format
    assert!(
        response.status().is_success(),
        "Response should be successful"
    );
}

async fn test_not_found(client: &Client, config: &Config) {
    info!("test_not_found");

    let url = format!("{}/not-found", &config.base_url);

    let response = client
        .get(&url)
        .send()
        .await
        .expect("Should be able to send request");

    assert!(
        response.status() == StatusCode::NOT_FOUND,
        "Response should be 404 Not Found"
    );

    // We should parse the protobuf body
    let body_bytes = response
        .bytes()
        .await
        .expect("Should be able to read response body");

    let error_message =
        ErrorMessageBuf::decode(&body_bytes[..]).expect("Should be able to decode error response");

    assert_eq!(error_message.status_code, 404, "Status code should be 404");
}

async fn test_setup(client: &Client, config: &Config) {
    info!("test_setup");

    let url = format!("{}/setup", &config.base_url);

    // Use a dummy data
    let body = SetupBodyBuf {
        setup_key: generate_id("sup"),
        email: "root@example.com".to_string(),
        password: "password".to_string(),
    };

    let response = client
        .post(&url)
        .body(prost::Message::encode_to_vec(&body))
        .header("Content-Type", "application/x-protobuf")
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Response should be 400 Bad Request"
    );

    // We should parse the protobuf body
    let body_bytes = response
        .bytes()
        .await
        .expect("Should be able to read response body");

    let error_message =
        ErrorMessageBuf::decode(&body_bytes[..]).expect("Should be able to decode error response");

    assert_eq!(error_message.status_code, 400, "Status code should be 400");
    assert_eq!(
        error_message.message, "Invalid setup key",
        "Error message should be 'Invalid setup key'"
    );
}

async fn test_health_liveness(client: &Client, config: &Config) {
    info!("test_health_liveness");

    let url = format!("{}/health/liveness", &config.base_url);

    let response = client
        .get(&url)
        .send()
        .await
        .expect("Should be able to send request");

    assert!(
        response.status().is_success(),
        "Response should be successful"
    );
}

async fn test_health_readiness(client: &Client, config: &Config) {
    info!("test_health_readiness");

    let url = format!("{}/health/readiness", &config.base_url);

    let response = client
        .get(&url)
        .send()
        .await
        .expect("Should be able to send request");

    assert!(
        response.status().is_success(),
        "Response should be successful"
    );
}
