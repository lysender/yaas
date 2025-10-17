use chrono::Utc;
use prost::Message;
use reqwest::{Client, StatusCode};
use tracing::info;

use crate::{TestActor, config::Config};
use yaas::{
    buffed::dto::{AppBuf, ErrorMessageBuf, NewAppBuf, PaginatedAppsBuf, UpdateAppBuf},
    dto::AppDto,
};

pub async fn run_tests(client: &Client, config: &Config, actor: &TestActor) {
    info!("Running apps tests");

    test_apps_listing(client, config, actor).await;
    test_apps_listing_unauthenticated(client, config).await;

    let app = test_create_app(client, config, actor).await;
    test_create_app_unauthenticated(client, config).await;

    test_get_app(client, config, actor, &app).await;
    test_get_app_not_found(client, config, actor).await;
    test_get_app_unauthenticated(client, config, &app).await;

    test_update_app_no_changes(client, config, actor, &app).await;
    test_update_app(client, config, actor, &app).await;
    test_update_app_name_only(client, config, actor, &app).await;
    test_update_app_unauthenticated(client, config, &app).await;

    test_delete_app(client, config, actor, &app).await;
    test_delete_app_not_found(client, config, actor).await;
    test_delete_app_unauthorized(client, config, &app).await;
}

async fn test_apps_listing(client: &Client, config: &Config, actor: &TestActor) {
    info!("test_apps_listing");

    let url = format!("{}/apps", &config.base_url);
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

    let listing = PaginatedAppsBuf::decode(&body_bytes[..])
        .expect("Should be able to decode PaginatedAppsBuf");

    // Apps may be empty, but meta should be present
    let meta = listing.meta.unwrap();
    assert!(meta.page == 1, "Page should be 1");
    assert!(meta.per_page == 50, "Per page should be 50");
}

async fn test_apps_listing_unauthenticated(client: &Client, config: &Config) {
    info!("test_apps_listing_unauthenticated");

    let url = format!("{}/apps", &config.base_url);
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

async fn test_create_app(client: &Client, config: &Config, actor: &TestActor) -> AppDto {
    info!("test_create_app");

    let random_pad = Utc::now().timestamp_millis();

    let name = format!("Test App {}", random_pad);

    let new_app = NewAppBuf {
        name: name.clone(),
        redirect_uri: "https://example.com/callback".to_string(),
    };

    let url = format!("{}/apps", &config.base_url);
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .body(new_app.encode_to_vec())
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

    let created_app = AppBuf::decode(&body_bytes[..]).expect("Should be able to decode AppBuf");
    let app_id = created_app.id;
    assert!(app_id > 0, "App ID should be greater than 0");
    assert_eq!(created_app.name, name, "Name should match");
    assert_eq!(
        &created_app.redirect_uri, "https://example.com/callback",
        "Redirect URI should match"
    );

    let dto: AppDto = created_app.into();
    dto
}

async fn test_create_app_unauthenticated(client: &Client, config: &Config) {
    info!("test_create_app_unauthenticated");

    let random_pad = Utc::now().timestamp_millis();

    let name = format!("Test App {}", random_pad);

    let new_app = NewAppBuf {
        name: name.clone(),
        redirect_uri: "https://example.com/callback".to_string(),
    };

    let url = format!("{}/apps", &config.base_url);
    let response = client
        .post(&url)
        .body(new_app.encode_to_vec())
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

async fn test_get_app(client: &Client, config: &Config, actor: &TestActor, app: &AppDto) {
    info!("test_get_app");

    let url = format!("{}/apps/{}", &config.base_url, app.id);
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

    let found_app = AppBuf::decode(&body_bytes[..]).expect("Should be able to decode AppBuf");
    assert_eq!(found_app.id, app.id, "App ID should match");
    assert_eq!(&found_app.name, &app.name, "Name should match");
    assert_eq!(
        found_app.redirect_uri, app.redirect_uri,
        "Redirect URI should match"
    );
}

async fn test_get_app_not_found(client: &Client, config: &Config, actor: &TestActor) {
    info!("test_get_app_not_found");

    let url = format!("{}/apps/{}", &config.base_url, 999999);
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

async fn test_get_app_unauthenticated(client: &Client, config: &Config, app: &AppDto) {
    info!("test_get_app_unauthenticated");

    let url = format!("{}/apps/{}", &config.base_url, app.id);
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

async fn test_update_app_no_changes(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    app: &AppDto,
) {
    info!("test_update_app_no_changes");

    let data = UpdateAppBuf {
        name: None,
        redirect_uri: None,
    };

    let url = format!("{}/apps/{}", &config.base_url, app.id);
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

    let updated_app = AppBuf::decode(&body_bytes[..]).expect("Should be able to decode AppBuf");
    assert_eq!(&updated_app.name, &app.name, "Name should be the same");
    assert_eq!(
        updated_app.redirect_uri, app.redirect_uri,
        "Redirect URI should be the same"
    );
}

async fn test_update_app(client: &Client, config: &Config, actor: &TestActor, app: &AppDto) {
    info!("test_update_app");

    let updated_name = format!("{} v2", app.name);

    let data = UpdateAppBuf {
        name: Some(updated_name.clone()),
        redirect_uri: Some("https://example.com/updated_callback".to_string()),
    };

    let url = format!("{}/apps/{}", &config.base_url, app.id);
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

    let updated_app = AppBuf::decode(&body_bytes[..]).expect("Should be able to decode AppBuf");
    assert_eq!(&updated_app.name, &updated_name, "Name should be updated");
    assert_eq!(
        &updated_app.redirect_uri, "https://example.com/updated_callback",
        "Redirect URI should be updated"
    );
}

async fn test_update_app_name_only(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    app: &AppDto,
) {
    info!("test_update_app_name_only");

    let data = UpdateAppBuf {
        name: Some(app.name.clone()),
        redirect_uri: None,
    };

    let url = format!("{}/apps/{}", &config.base_url, app.id);
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

    let updated_app = AppBuf::decode(&body_bytes[..]).expect("Should be able to decode AppBuf");
    assert_eq!(
        &updated_app.name, &app.name,
        "Name should be reverted back to original"
    );
    // Should be equal to the previous update
    assert_eq!(
        &updated_app.redirect_uri, "https://example.com/updated_callback",
        "Redirect URI should be unchanged"
    );
}

async fn test_update_app_unauthenticated(client: &Client, config: &Config, app: &AppDto) {
    info!("test_update_app_unauthenticated");

    let data = UpdateAppBuf {
        name: None,
        redirect_uri: None,
    };

    let url = format!("{}/apps/{}", &config.base_url, app.id);
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
        ErrorMessageBuf::decode(&body_bytes[..]).expect("Should be able to decode ErrorMessageBuf");
    assert_eq!(
        error_message.status_code, 401,
        "Error status code should be 401 Unauthorized"
    );
}

async fn test_delete_app(client: &Client, config: &Config, actor: &TestActor, app: &AppDto) {
    info!("test_delete_app");

    let url = format!("{}/apps/{}", &config.base_url, app.id);
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

async fn test_delete_app_not_found(client: &Client, config: &Config, actor: &TestActor) {
    info!("test_delete_app_not_found");

    let url = format!("{}/apps/{}", &config.base_url, 999999);
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

async fn test_delete_app_unauthorized(client: &Client, config: &Config, app: &AppDto) {
    info!("test_delete_app_unauthorized");

    let url = format!("{}/apps/{}", &config.base_url, app.id);
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
