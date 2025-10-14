use chrono::Utc;
use prost::Message;
use reqwest::{Client, StatusCode};
use tracing::info;

use crate::config::Config;
use yaas::{
    buffed::dto::{
        AppBuf, ErrorMessageBuf, NewAppBuf, NewOrgAppBuf, NewOrgBuf, NewUserWithPasswordBuf,
        OrgAppBuf, OrgBuf, PaginatedOrgAppsBuf, UserBuf,
    },
    dto::{AppDto, OrgAppDto, OrgDto, UserDto},
};

pub async fn run_tests(client: &Client, config: &Config, token: &str) {
    info!("Running org apps tests");

    // Need a user to own the org
    let admin_user = create_test_user(client, config, token).await;

    // Need an org to work with
    let org = create_test_org(client, config, token, &admin_user).await;

    // Need a test apps to work with
    let app = create_test_app(client, config, token).await;

    let org_app = create_test_org_app(client, config, token, &org, &app).await;
    test_create_org_app_not_exists(client, config, token, &org).await;
    test_create_org_app_already_exists(client, config, token, &org, &app).await;
    test_create_org_app_unauthenticated(client, config, &org, &app).await;

    test_org_apps_listing(client, config, token, &org).await;
    test_org_apps_listing_unauthenticated(client, config, &org).await;

    test_get_org_app(client, config, token, &org_app).await;
    test_get_org_app_not_found(client, config, token, &org).await;
    test_get_org_app_unauthenticated(client, config, &org_app).await;

    test_delete_org_app_not_found(client, config, token, &org).await;
    test_delete_org_app_unauthorized(client, config, &org_app).await;
    test_delete_org_app(client, config, token, &org_app).await;

    // Cleanup created resources
    delete_test_org(client, config, token, &org).await;
    delete_test_user(client, config, token, &admin_user).await;
    delete_test_app(client, config, token, &app).await;
}

async fn test_org_apps_listing(client: &Client, config: &Config, token: &str, org: &OrgDto) {
    info!("test_org_apps_listing");

    let url = format!("{}/orgs/{}/apps", &config.base_url, org.id);
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

    let listing = PaginatedOrgAppsBuf::decode(&body_bytes[..])
        .expect("Should be able to decode PaginatedOrgAppsBuf");

    let meta = listing.meta.unwrap();
    assert!(meta.page == 1, "Page should be 1");
    assert!(meta.per_page == 50, "Per page should be 50");
    assert!(meta.total_records >= 1, "Total records should be >= 1");
    assert!(meta.total_pages >= 1, "Total pages should be >= 1");

    assert!(listing.data.len() >= 1, "There should be at least one user");

    // Each members should belong to the org
    for member in listing.data.iter() {
        assert!(member.org_id == org.id, "Member should belong to the org");
    }
}

async fn test_org_apps_listing_unauthenticated(client: &Client, config: &Config, org: &OrgDto) {
    info!("test_org_apps_listing_unauthenticated");

    let url = format!("{}/orgs/{}/apps", &config.base_url, org.id);
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

async fn create_test_user(client: &Client, config: &Config, token: &str) -> UserDto {
    info!("create_test_user");

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

async fn create_test_org(client: &Client, config: &Config, token: &str, owner: &UserDto) -> OrgDto {
    info!("create_test_org");

    let random_pad = Utc::now().timestamp_millis();

    let name = format!("Test Org {}", random_pad);

    let new_org = NewOrgBuf {
        name: name.clone(),
        owner_id: owner.id,
    };

    let url = format!("{}/orgs", &config.base_url);
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .body(new_org.encode_to_vec())
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

    let created_org = OrgBuf::decode(&body_bytes[..]).expect("Should be able to decode OrgBuf");
    let org_id = created_org.id;
    assert!(org_id > 0, "User ID should be greater than 0");
    assert_eq!(created_org.name, name, "Name should match");
    assert_eq!(
        created_org.owner_id,
        Some(owner.id),
        "Owner ID should match"
    );

    let dto: OrgDto = created_org.into();
    dto
}

async fn create_test_app(client: &Client, config: &Config, token: &str) -> AppDto {
    info!("create_test_app");

    let random_pad = Utc::now().timestamp_millis();

    let name = format!("Test App {}", random_pad);

    let new_app = NewAppBuf {
        name: name.clone(),
        redirect_uri: "https://example.com/callback".to_string(),
    };

    let url = format!("{}/apps", &config.base_url);
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
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

async fn create_test_org_app(
    client: &Client,
    config: &Config,
    token: &str,
    org: &OrgDto,
    app: &AppDto,
) -> OrgAppDto {
    info!("create_test_org_app");

    let new_org_app = NewOrgAppBuf { app_id: app.id };

    let url = format!("{}/orgs/{}/apps", &config.base_url, org.id);
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .body(new_org_app.encode_to_vec())
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

    let created_org_app =
        OrgAppBuf::decode(&body_bytes[..]).expect("Should be able to decode OrgAppBuf");

    let org_app_id = created_org_app.id;
    assert!(org_app_id > 0, "Org App ID should be greater than 0");
    assert_eq!(created_org_app.org_id, org.id, "Org ID should match");
    assert_eq!(created_org_app.app_id, app.id, "App ID should match");

    let dto: OrgAppDto = created_org_app.into();
    dto
}

async fn test_create_org_app_not_exists(
    client: &Client,
    config: &Config,
    token: &str,
    org: &OrgDto,
) {
    info!("test_create_org_app_not_exists");

    let new_org_app = NewOrgAppBuf { app_id: 99999 };

    let url = format!("{}/orgs/{}/apps", &config.base_url, org.id);
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .body(new_org_app.encode_to_vec())
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

async fn test_create_org_app_already_exists(
    client: &Client,
    config: &Config,
    token: &str,
    org: &OrgDto,
    app: &AppDto,
) {
    info!("test_create_org_app_already_exists");

    let new_org_app = NewOrgAppBuf { app_id: app.id };

    let url = format!("{}/orgs/{}/apps", &config.base_url, org.id);
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .body(new_org_app.encode_to_vec())
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

async fn test_create_org_app_unauthenticated(
    client: &Client,
    config: &Config,
    org: &OrgDto,
    app: &AppDto,
) {
    info!("test_create_org_app_unauthenticated");

    let new_org_app = NewOrgAppBuf { app_id: app.id };

    let url = format!("{}/orgs/{}/apps", &config.base_url, org.id);
    let response = client
        .post(&url)
        .body(new_org_app.encode_to_vec())
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

async fn test_get_org_app(client: &Client, config: &Config, token: &str, org_app: &OrgAppDto) {
    info!("test_get_org_app");

    let url = format!(
        "{}/orgs/{}/apps/{}",
        &config.base_url, org_app.org_id, org_app.app_id
    );
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

    let found_org_app =
        OrgAppBuf::decode(&body_bytes[..]).expect("Should be able to decode OrgAppBuf");

    assert_eq!(found_org_app.id, org_app.id, "Org App ID should match");
    assert_eq!(found_org_app.org_id, org_app.org_id, "Org ID should match");
    assert_eq!(found_org_app.app_id, org_app.app_id, "App ID should match");
    assert_eq!(
        &found_org_app.app_name, &org_app.app_name,
        "Name should match"
    );
}

async fn test_get_org_app_not_found(client: &Client, config: &Config, token: &str, org: &OrgDto) {
    info!("test_get_org_app_not_found");

    let url = format!("{}/orgs/{}/apps/{}", &config.base_url, org.id, 999999);
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

async fn test_get_org_app_unauthenticated(client: &Client, config: &Config, org_app: &OrgAppDto) {
    info!("test_get_org_app_unauthenticated");

    let url = format!(
        "{}/orgs/{}/apps/{}",
        &config.base_url, org_app.org_id, org_app.app_id
    );
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

async fn test_delete_org_app(client: &Client, config: &Config, token: &str, org_app: &OrgAppDto) {
    info!("test_delete_org_app");

    let url = format!(
        "{}/orgs/{}/apps/{}",
        &config.base_url, org_app.org_id, org_app.app_id
    );
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

async fn test_delete_org_app_not_found(
    client: &Client,
    config: &Config,
    token: &str,
    org: &OrgDto,
) {
    info!("test_delete_org_app_not_found");

    let url = format!("{}/orgs/{}/apps/{}", &config.base_url, org.id, 999999);
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

async fn test_delete_org_app_unauthorized(client: &Client, config: &Config, org_app: &OrgAppDto) {
    info!("test_delete_org_app_unauthorized");

    let url = format!(
        "{}/orgs/{}/apps/{}",
        &config.base_url, org_app.org_id, org_app.app_id
    );
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

async fn delete_test_user(client: &Client, config: &Config, token: &str, user: &UserDto) {
    info!("delete_test_user");

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

async fn delete_test_org(client: &Client, config: &Config, token: &str, org: &OrgDto) {
    info!("delete_test_org");

    let url = format!("{}/orgs/{}", &config.base_url, org.id);
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

async fn delete_test_app(client: &Client, config: &Config, token: &str, app: &AppDto) {
    info!("delete_test_app");

    let url = format!("{}/apps/{}", &config.base_url, app.id);
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
