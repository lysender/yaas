use chrono::Utc;
use prost::Message;
use reqwest::{Client, StatusCode};
use tracing::info;

use crate::config::Config;
use yaas::{
    buffed::dto::{ErrorMessageBuf, NewUserWithPasswordBuf, PaginatedUsersBuf, UserBuf},
    dto::UserDto,
};

pub async fn run_tests(client: &Client, config: &Config, token: &str) {
    info!("Running users tests");

    test_users_listing(client, config, token).await;
    test_users_listing_unauthenticated(client, config).await;

    let user = test_create_user(client, config, token).await;
    test_create_user_unauthenticated(client, config).await;

    test_get_user(client, config, token, &user).await;
    test_get_user_not_found(client, config, token).await;
    test_get_user_unauthenticated(client, config, &user).await;

    test_delete_user(client, config, token, &user).await;
    test_delete_user_not_found(client, config, token).await;
    test_delete_user_unauthorized(client, config, &user).await;
}

async fn test_users_listing(client: &Client, config: &Config, token: &str) {
    info!("test_users_listing");

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
    info!("test_users_listing_unauthenticated");

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

/// Creates a new, gets it, deletes it, and ensures it's gone
async fn test_create_user(client: &Client, config: &Config, token: &str) -> UserDto {
    info!("test_create_user");

    let random_pad = Utc::now().timestamp_millis();

    let email = format!("testuser.{}@example.com", random_pad);
    let name = format!("Test User {}", random_pad);
    let password = "password".to_string();

    let new_user = NewUserWithPasswordBuf {
        email: email.clone(),
        name: name.clone(),
        password: password.clone(),
    };

    let url = format!("{}/users", &config.base_url);
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .body(new_user.encode_to_vec())
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "Response should be 201 Created"
    );

    let body_bytes = response
        .bytes()
        .await
        .expect("Should be able to read response body");

    // After created, now what? Delete it?
    let created_user = UserBuf::decode(&body_bytes[..]).expect("Should be able to decode UserBuf");
    let user_id = created_user.id;
    assert!(user_id > 0, "User ID should be greater than 0");
    assert_eq!(created_user.email, email, "Email should match");
    assert_eq!(created_user.name, name, "Name should match");

    let dto: UserDto = created_user.into();
    dto
}

async fn test_get_user(client: &Client, config: &Config, token: &str, user: &UserDto) {
    info!("test_get_user");

    let url = format!("{}/users/{}", &config.base_url, user.id);
    let response = client
        .get(&url)
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

    let found_user = UserBuf::decode(&body_bytes[..]).expect("Should be able to decode UserBuf");
    assert_eq!(found_user.id, user.id, "User ID should match");
    assert_eq!(&found_user.email, &user.email, "Email should match");
}

async fn test_get_user_not_found(client: &Client, config: &Config, token: &str) {
    info!("test_get_user_not_found");

    let url = format!("{}/users/{}", &config.base_url, 999999);
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        response.status(),
        StatusCode::NOT_FOUND,
        "Response should be 404 Not Found"
    );

    let body_bytes = response
        .bytes()
        .await
        .expect("Should be able to read response body");

    let error_message =
        ErrorMessageBuf::decode(&body_bytes[..]).expect("Should be able to decode ErrorMessageBuf");
    assert_eq!(
        error_message.status_code, 404,
        "Error status code should be 404 Not Found"
    );
}

async fn test_get_user_unauthenticated(client: &Client, config: &Config, user: &UserDto) {
    info!("test_get_user_unauthenticated");

    let url = format!("{}/users/{}", &config.base_url, user.id);
    let response = client
        .get(&url)
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

async fn test_create_user_unauthenticated(client: &Client, config: &Config) {
    info!("test_create_user_unauthenticated");

    let random_pad = Utc::now().timestamp_millis();

    let email = format!("testuser.{}@example.com", random_pad);
    let name = format!("Test User {}", random_pad);
    let password = "password".to_string();

    let new_user = NewUserWithPasswordBuf {
        email: email.clone(),
        name: name.clone(),
        password: password.clone(),
    };

    let url = format!("{}/users", &config.base_url);
    let response = client
        .post(url)
        .body(new_user.encode_to_vec())
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

async fn test_delete_user(client: &Client, config: &Config, token: &str, user: &UserDto) {
    info!("test_delete_user");

    let url = format!("{}/users/{}", &config.base_url, user.id);
    let delete_response = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        delete_response.status(),
        StatusCode::NO_CONTENT,
        "Response should be 204 No Content"
    );

    let body_bytes = delete_response
        .bytes()
        .await
        .expect("Should be able to read response body");

    assert_eq!(body_bytes.len(), 0, "Response body should be empty");

    // Get it again, should be gone
    let get_response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        get_response.status(),
        StatusCode::NOT_FOUND,
        "Response should be 404 Not Found"
    );

    let body_bytes = get_response
        .bytes()
        .await
        .expect("Should be able to read response body");

    let error_message =
        ErrorMessageBuf::decode(&body_bytes[..]).expect("Should be able to decode ErrorMessageBuf");
    assert_eq!(
        error_message.status_code, 404,
        "Error status code should be 404 Not Found"
    );
}

async fn test_delete_user_not_found(client: &Client, config: &Config, token: &str) {
    info!("test_delete_user_not_found");

    let url = format!("{}/users/{}", &config.base_url, 999999);
    let delete_response = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        delete_response.status(),
        StatusCode::NOT_FOUND,
        "Response should be 404 Not Found"
    );

    let body_bytes = delete_response
        .bytes()
        .await
        .expect("Should be able to read response body");

    let error_message =
        ErrorMessageBuf::decode(&body_bytes[..]).expect("Should be able to decode ErrorMessageBuf");
    assert_eq!(
        error_message.status_code, 404,
        "Error status code should be 404 Not Found"
    );
}

async fn test_delete_user_unauthorized(client: &Client, config: &Config, user: &UserDto) {
    info!("test_delete_user_unauthorized");

    let url = format!("{}/users/{}", &config.base_url, user.id);
    let delete_response = client
        .delete(&url)
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        delete_response.status(),
        StatusCode::UNAUTHORIZED,
        "Response should be 401 Unauthorized"
    );

    let body_bytes = delete_response
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
