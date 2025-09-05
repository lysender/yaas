use prost::Message;
use reqwest::{Client, StatusCode};
use tracing::info;

use yaas::buffed::actor::{ActorBuf, AuthResponseBuf, CredentialsBuf};
use yaas::buffed::dto::{ChangeCurrentPasswordBuf, ErrorMessageBuf, UserBuf};

use crate::config::Config;

pub async fn run_tests(client: &Client, config: &Config) {
    info!("Running user tests");

    let superuser_token = authenticate_superuser(client, config).await;

    test_user_profile(client, config, &superuser_token).await;
    test_user_profile_unauthenticated(client, config).await;

    test_user_authz(client, config, &superuser_token).await;
    test_user_authz_unauthenticated(client, config).await;

    test_user_change_password(client, config, &superuser_token).await;
    test_user_change_password_incorrect(client, config, &superuser_token).await;
    test_user_change_password_unauthenticated(client, config).await;
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

async fn test_user_profile(client: &Client, config: &Config, token: &str) {
    info!("test_user_profile");

    let url = format!("{}/user", &config.base_url);
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

    let user = UserBuf::decode(&body_bytes[..]).expect("Should be able to decode UserBuf");

    assert_eq!(
        user.email, config.superuser_email,
        "User email should match"
    );
}

async fn test_user_profile_unauthenticated(client: &Client, config: &Config) {
    info!("test_user_profile_unauthenticated");

    let url = format!("{}/user", &config.base_url);
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

async fn test_user_authz(client: &Client, config: &Config, token: &str) {
    info!("test_user_authz");

    let url = format!("{}/user/authz", &config.base_url);
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

    let actor = ActorBuf::decode(&body_bytes[..]).expect("Should be able to decode ActorBuf");

    assert!(actor.user.is_some(), "Actor should contain a user");
    assert!(actor.roles.len() > 0, "Actor should have roles");
    assert!(actor.permissions.len() > 0, "Actor should have permissions");
}

async fn test_user_authz_unauthenticated(client: &Client, config: &Config) {
    info!("test_user_authz_unauthenticated");

    let url = format!("{}/user/authz", &config.base_url);
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

async fn test_user_change_password(client: &Client, config: &Config, token: &str) {
    info!("test_user_change_password");

    let url = format!("{}/user/change-password", &config.base_url);
    let body = ChangeCurrentPasswordBuf {
        current_password: config.superuser_password.clone(),
        new_password: config.superuser_password.clone(),
    };

    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", token))
        .body(prost::Message::encode_to_vec(&body))
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        response.status(),
        StatusCode::NO_CONTENT,
        "Response should be 204 No Content"
    );

    let body_bytes = response
        .bytes()
        .await
        .expect("Should be able to read revert response body");

    assert!(
        body_bytes.len() == 0,
        "Revert response body should be empty"
    );
}

async fn test_user_change_password_incorrect(client: &Client, config: &Config, token: &str) {
    info!("test_user_change_password_incorrect");

    let url = format!("{}/user/change-password", &config.base_url);
    let body = ChangeCurrentPasswordBuf {
        current_password: "wrongpassword".to_string(),
        new_password: "newpassword".to_string(),
    };

    let response = client
        .post(url)
        .header("Authorization", format!("Bearer {}", token))
        .body(prost::Message::encode_to_vec(&body))
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
    assert_eq!(
        error_message.message, "Current password is incorrect",
        "Error message should indicate incorrect current password"
    );
}

async fn test_user_change_password_unauthenticated(client: &Client, config: &Config) {
    info!("test_user_change_password_unauthenticated");

    let url = format!("{}/user/change-password", &config.base_url);
    let body = ChangeCurrentPasswordBuf {
        current_password: "password".to_string(),
        new_password: "newpassword".to_string(),
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
