use chrono::Utc;
use prost::Message;
use reqwest::{Client, StatusCode};
use tracing::info;

use crate::{TestActor, config::Config};
use yaas::{
    buffed::dto::{
        ErrorMessageBuf, NewPasswordBuf, NewUserWithPasswordBuf, PaginatedUsersBuf, UpdateUserBuf,
        UserBuf,
    },
    dto::UserDto,
};

pub async fn run_tests(client: &Client, config: &Config, actor: &TestActor) {
    info!("Running users tests");

    let user = test_create_user(client, config, actor).await;
    test_create_user_unauthenticated(client, config).await;

    test_users_listing(client, config, actor, &user).await;
    test_users_listing_unauthenticated(client, config).await;

    test_get_user(client, config, actor, &user).await;
    test_get_user_not_found(client, config, actor).await;
    test_get_user_unauthenticated(client, config, &user).await;

    test_update_user_no_changes(client, config, actor, &user).await;
    test_update_user(client, config, actor, &user).await;
    test_update_user_name_only(client, config, actor, &user).await;
    test_update_user_unauthenticated(client, config, &user).await;

    test_update_user_password(client, config, actor, &user).await;
    test_update_user_password_empty(client, config, actor, &user).await;
    test_update_user_password_unauthenticated(client, config, &user).await;

    test_delete_user(client, config, actor, &user).await;
    test_delete_user_not_found(client, config, actor).await;
    test_delete_user_unauthorized(client, config, &user).await;
}

async fn test_users_listing(client: &Client, config: &Config, actor: &TestActor, user: &UserDto) {
    info!("test_users_listing");

    let url = format!("{}/users", &config.base_url);
    let response = client
        .get(url)
        .header("Authorization", format!("Bearer {}", &actor.token))
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

    assert!(
        !listing.data.is_empty(),
        "There should be at least one user"
    );

    // User must be in the listing
    let found = listing.data.iter().find(|u| u.id == user.id);
    assert!(found.is_some(), "Created user should be in the listing");
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

async fn test_create_user(client: &Client, config: &Config, actor: &TestActor) -> UserDto {
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
        .header("Authorization", format!("Bearer {}", &actor.token))
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

async fn test_get_user(client: &Client, config: &Config, actor: &TestActor, user: &UserDto) {
    info!("test_get_user");

    let url = format!("{}/users/{}", &config.base_url, user.id);
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
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

async fn test_get_user_not_found(client: &Client, config: &Config, actor: &TestActor) {
    info!("test_get_user_not_found");

    let url = format!("{}/users/{}", &config.base_url, 999999);
    let response = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
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

async fn test_update_user_no_changes(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    user: &UserDto,
) {
    info!("test_update_user_no_changes");

    let data = UpdateUserBuf {
        name: None,
        status: None,
    };

    let url = format!("{}/users/{}", &config.base_url, user.id);
    let response = client
        .patch(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .body(data.encode_to_vec())
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

    let updated_user = UserBuf::decode(&body_bytes[..]).expect("Should be able to decode UserBuf");
    assert_eq!(&updated_user.name, &user.name, "Name should be the same");
    assert_eq!(
        &updated_user.status, &user.status,
        "Status should be the same"
    );
}

async fn test_update_user(client: &Client, config: &Config, actor: &TestActor, user: &UserDto) {
    info!("test_update_user");

    let updated_name = format!("{} v2", user.name);
    let updated_status = "inactive".to_string();

    let data = UpdateUserBuf {
        name: Some(updated_name.clone()),
        status: Some(updated_status.clone()),
    };

    let url = format!("{}/users/{}", &config.base_url, user.id);
    let response = client
        .patch(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .body(data.encode_to_vec())
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

    let updated_user = UserBuf::decode(&body_bytes[..]).expect("Should be able to decode UserBuf");
    assert_eq!(&updated_user.name, &updated_name, "Name should be updated");
    assert_eq!(
        &updated_user.status, &updated_status,
        "Status should be updated"
    );
}

async fn test_update_user_name_only(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    user: &UserDto,
) {
    info!("test_update_user_status_only");

    let data = UpdateUserBuf {
        name: Some(user.name.clone()),
        status: None,
    };

    let url = format!("{}/users/{}", &config.base_url, user.id);
    let response = client
        .patch(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .body(data.encode_to_vec())
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

    let updated_user = UserBuf::decode(&body_bytes[..]).expect("Should be able to decode UserBuf");
    assert_eq!(
        &updated_user.name, &user.name,
        "Name should be reverted back to original"
    );
    assert_eq!(
        &updated_user.status, "inactive",
        "Status should be still be the same"
    );
}

async fn test_update_user_password(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    user: &UserDto,
) {
    info!("test_update_user_password");

    let data = NewPasswordBuf {
        password: "newpassword".to_string(),
    };

    let url = format!("{}/users/{}/password", &config.base_url, user.id);
    let response = client
        .put(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .body(data.encode_to_vec())
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
        .expect("Should be able to read response body");

    assert_eq!(body_bytes.len(), 0, "Response body should be empty");
}

async fn test_update_user_password_empty(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    user: &UserDto,
) {
    info!("test_update_user_password_empty");

    let data = NewPasswordBuf {
        password: "".to_string(),
    };

    let url = format!("{}/users/{}/password", &config.base_url, user.id);
    let response = client
        .put(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .body(data.encode_to_vec())
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
        ErrorMessageBuf::decode(&body_bytes[..]).expect("Should be able to decode UserBuf");
    assert_eq!(
        error_message.status_code, 400,
        "Error status code should be 400 Bad Request"
    );
}

async fn test_update_user_password_unauthenticated(
    client: &Client,
    config: &Config,
    user: &UserDto,
) {
    info!("test_update_user_password_unauthenticated");

    let data = NewPasswordBuf {
        password: "newpassword".to_string(),
    };

    let url = format!("{}/users/{}/password", &config.base_url, user.id);
    let response = client
        .put(&url)
        .body(data.encode_to_vec())
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
        ErrorMessageBuf::decode(&body_bytes[..]).expect("Should be able to decode UserBuf");
    assert_eq!(
        error_message.status_code, 401,
        "Error status code should be 401 Unauthorized"
    );
}

async fn test_update_user_unauthenticated(client: &Client, config: &Config, user: &UserDto) {
    info!("test_update_user_unauthenticated");

    let data = UpdateUserBuf {
        name: None,
        status: None,
    };

    let url = format!("{}/users/{}", &config.base_url, user.id);
    let response = client
        .patch(&url)
        .body(data.encode_to_vec())
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
        ErrorMessageBuf::decode(&body_bytes[..]).expect("Should be able to decode UserBuf");
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

async fn test_delete_user(client: &Client, config: &Config, actor: &TestActor, user: &UserDto) {
    info!("test_delete_user");

    let url = format!("{}/users/{}", &config.base_url, user.id);
    let delete_response = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
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
        .header("Authorization", format!("Bearer {}", &actor.token))
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

async fn test_delete_user_not_found(client: &Client, config: &Config, actor: &TestActor) {
    info!("test_delete_user_not_found");

    let url = format!("{}/users/{}", &config.base_url, 999999);
    let delete_response = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
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
