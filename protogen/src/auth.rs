use prost::Message;
use reqwest::{Client, StatusCode};
use tracing::info;

use yaas::buffed::actor::{AuthResponseBuf, CredentialsBuf};
use yaas::buffed::dto::ErrorMessageBuf;

use crate::config::Config;

pub async fn run_tests(client: &Client, config: &Config) {
    info!("Running auth tests");

    test_no_body(client, config).await;
    test_invalid_credentials(client, config).await;
    test_valid_credentials(client, config).await;
}

async fn test_no_body(client: &Client, config: &Config) {
    info!("test_no_body");

    let url = format!("{}/auth/authorize", &config.base_url);
    let response = client
        .post(url)
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        response.status(),
        StatusCode::BAD_REQUEST,
        "Response should be 400 Bad Request"
    );

    let body_bytes = response
        .bytes()
        .await
        .expect("Should be able to read response body");

    let error_message =
        ErrorMessageBuf::decode(&body_bytes[..]).expect("Should be able to decode ErrorMessageBuf");

    assert_eq!(
        error_message.status_code, 400,
        "Error status code should be 400 Bad Request"
    );
}

async fn test_invalid_credentials(client: &Client, config: &Config) {
    info!("test_invalid_credentials");

    let url = format!("{}/auth/authorize", &config.base_url);
    let body = CredentialsBuf {
        email: "foo@example.com".to_string(),
        password: "wrongpassword".to_string(),
    };

    let response = client
        .post(url)
        .body(prost::Message::encode_to_vec(&body))
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

async fn test_valid_credentials(client: &Client, config: &Config) {
    info!("test_valid_credentials");

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
        auth_response.token.len() > 0,
        "AuthResponse should contain a token"
    );
    assert!(
        auth_response.org_id > 0,
        "AuthResponse should contain org_id"
    );
    assert!(
        auth_response.org_count > 0,
        "AuthResponse should contain org_count"
    );
}
