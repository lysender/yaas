use reqwest::{Client, StatusCode};
use tracing::info;

use yaas::dto::{ErrorMessageDto, SetupBodyDto, SetupStatusDto};
use yaas::utils::{IdPrefix, generate_id};

use crate::config::Config;

pub async fn run_tests(client: &Client, config: &Config) {
    info!("Running smoke tests");

    test_home(client, config).await;
    test_not_found(client, config).await;
    test_setup_status(client, config).await;
    test_setup(client, config).await;
    test_health_liveness(client, config).await;
    test_health_readiness(client, config).await;
}

async fn test_setup_status(client: &Client, config: &Config) {
    info!("test_setup_status");

    let url = format!("{}/setup", &config.base_url);

    let response = client
        .get(&url)
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Response should be 200 OK"
    );

    let setup_status = response
        .json::<SetupStatusDto>()
        .await
        .expect("Should be able to decode setup status response");

    assert!(
        setup_status.done,
        "Setup status should report done=true when superuser exists"
    );
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

    let error_message = response
        .json::<ErrorMessageDto>()
        .await
        .expect("Should be able to decode error response");

    assert_eq!(error_message.status_code, 404, "Status code should be 404");
}

async fn test_setup(client: &Client, config: &Config) {
    info!("test_setup");

    let url = format!("{}/setup", &config.base_url);

    // Use a dummy data
    let body = SetupBodyDto {
        setup_key: generate_id(IdPrefix::Superuser),
        email: "root@example.com".to_string(),
        password: "password".to_string(),
    };

    let response = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Response should be 400 Bad Request"
    );

    let error_message = response
        .json::<ErrorMessageDto>()
        .await
        .expect("Should be able to decode error response");

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
