use prost::Message;
use reqwest::{Client, StatusCode};
use tracing::info;

use yaas::buffed::dto::{ErrorMessageBuf, SetupBodyBuf};
use yaas::utils::generate_id;

pub async fn run_tests(client: Client, base_url: &str) {
    info!("Running smoke tests...");

    test_home(&client, base_url).await;
    test_not_found(&client, base_url).await;
    test_setup(&client, base_url).await;
    test_health_liveness(&client, base_url).await;
    test_health_readiness(&client, base_url).await;
}

async fn test_home(client: &Client, base_url: &str) {
    info!("test_home...");

    let response = client
        .get(base_url)
        .send()
        .await
        .expect("Should be able to send request");

    // TODO: Body should be in protobuf format
    assert!(
        response.status().is_success(),
        "Response should be successful"
    );
}

async fn test_not_found(client: &Client, base_url: &str) {
    info!("test_not_found...");

    let url = format!("{}/not-found", base_url);

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

async fn test_setup(client: &Client, base_url: &str) {
    info!("test_setup...");

    let url = format!("{}/setup", base_url);

    // Use a dummy data
    let body = SetupBodyBuf {
        setup_key: generate_id("sup"),
        email: "root@lysender.com".to_string(),
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

async fn test_health_liveness(client: &Client, base_url: &str) {
    info!("test_health_liveness...");

    let url = format!("{}/health/liveness", base_url);

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

async fn test_health_readiness(client: &Client, base_url: &str) {
    info!("test_health_readiness...");

    let url = format!("{}/health/readiness", base_url);

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
