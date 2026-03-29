use chrono::Utc;
use reqwest::{Client, StatusCode};
use tracing::info;

use crate::{TestActor, config::Config};
use yaas::{
    dto::{AppDto, ErrorMessageDto, OrgAppDto, OrgDto, UserDto},
    pagination::Paginated,
};

pub async fn run_tests(client: &Client, config: &Config, actor: &TestActor) {
    info!("Running org apps tests");

    // Need a user to own the org
    let admin_user = create_test_user(client, config, actor).await;

    // Need an org to work with
    let org = create_test_org(client, config, actor, &admin_user).await;

    // Need a test apps to work with
    let app = create_test_app(client, config, actor).await;

    let org_app = create_test_org_app(client, config, actor, &org, &app).await;
    test_create_org_app_not_exists(client, config, actor, &org).await;
    test_create_org_app_already_exists(client, config, actor, &org, &app).await;
    test_create_org_app_unauthenticated(client, config, &org, &app).await;

    test_org_apps_listing(client, config, actor, &org).await;
    test_org_apps_listing_unauthenticated(client, config, &org).await;

    test_get_org_app(client, config, actor, &org_app).await;
    test_get_org_app_not_found(client, config, actor, &org).await;
    test_get_org_app_unauthenticated(client, config, &org_app).await;

    test_delete_org_app_not_found(client, config, actor, &org).await;
    test_delete_org_app_unauthorized(client, config, &org_app).await;
    test_delete_org_app(client, config, actor, &org_app).await;

    // Cleanup created resources
    delete_test_org(client, config, actor, &org).await;
    delete_test_user(client, config, actor, &admin_user).await;
    delete_test_app(client, config, actor, &app).await;
}

async fn test_org_apps_listing(client: &Client, config: &Config, actor: &TestActor, org: &OrgDto) {
    info!("test_org_apps_listing");

    let url = format!("{}/orgs/{}/apps", &config.base_url, org.id);
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

    let listing = response
        .json::<Paginated<OrgAppDto>>()
        .await
        .expect("Should be able to decode paginated org apps response");

    let meta = listing.meta;
    assert!(meta.page == 1, "Page should be 1");
    assert!(meta.per_page == 50, "Per page should be 50");
    assert!(meta.total_records >= 1, "Total records should be >= 1");
    assert!(meta.total_pages >= 1, "Total pages should be >= 1");

    assert!(
        !listing.data.is_empty(),
        "There should be at least one user"
    );

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

    let error_message = response
        .json::<ErrorMessageDto>()
        .await
        .expect("Should be able to decode ErrorMessageDto");

    assert_eq!(
        error_message.status_code, 401,
        "Error status code should be 401 Unauthorized"
    );
}

async fn create_test_user(client: &Client, config: &Config, actor: &TestActor) -> UserDto {
    info!("create_test_user");

    let random_pad = Utc::now().timestamp_millis();

    let email = format!("testuser.{}@example.com", random_pad);
    let name = format!("Test User {}", random_pad);
    let password = "password".to_string();

    let new_user = serde_json::json!({
        "email": email,
        "name": name,
        "password": password,
    });

    let url = format!("{}/users", &config.base_url);
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .json(&new_user)
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "Response should be 201 Created"
    );

    let created_user = response
        .json::<UserDto>()
        .await
        .expect("Should be able to decode UserDto");
    let user_id = created_user.id.clone();
    assert!(!user_id.is_empty(), "User ID should not be empty");
    assert_eq!(created_user.email, email, "Email should match");
    assert_eq!(created_user.name, name, "Name should match");

    created_user
}

async fn create_test_org(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    owner: &UserDto,
) -> OrgDto {
    info!("create_test_org");

    let random_pad = Utc::now().timestamp_millis();

    let name = format!("Test Org {}", random_pad);

    let new_org = serde_json::json!({
        "name": name,
        "owner_id": owner.id,
    });

    let url = format!("{}/orgs", &config.base_url);
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .json(&new_org)
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "Response should be 201 Created"
    );

    let created_org = response
        .json::<OrgDto>()
        .await
        .expect("Should be able to decode OrgDto");
    let org_id = created_org.id.clone();
    assert!(!org_id.is_empty(), "Org ID should not be empty");
    assert_eq!(created_org.name, name, "Name should match");
    assert_eq!(
        created_org.owner_id,
        Some(owner.id.clone()),
        "Owner ID should match"
    );

    created_org
}

async fn create_test_app(client: &Client, config: &Config, actor: &TestActor) -> AppDto {
    info!("create_test_app");

    let random_pad = Utc::now().timestamp_millis();

    let name = format!("Test App {}", random_pad);

    let new_app = serde_json::json!({
        "name": name,
        "redirect_uri": "https://example.com/callback",
    });

    let url = format!("{}/apps", &config.base_url);
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .json(&new_app)
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "Response should be 201 Created"
    );

    let created_app = response
        .json::<AppDto>()
        .await
        .expect("Should be able to decode AppDto");
    let app_id = created_app.id.clone();
    assert!(!app_id.is_empty(), "App ID should not be empty");
    assert_eq!(created_app.name, name, "Name should match");
    assert_eq!(
        &created_app.redirect_uri, "https://example.com/callback",
        "Redirect URI should match"
    );

    created_app
}

async fn create_test_org_app(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    org: &OrgDto,
    app: &AppDto,
) -> OrgAppDto {
    info!("create_test_org_app");

    let new_org_app = serde_json::json!({
        "app_id": app.id,
    });

    let url = format!("{}/orgs/{}/apps", &config.base_url, org.id);
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .json(&new_org_app)
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        response.status(),
        StatusCode::CREATED,
        "Response should be 201 Created"
    );

    let created_org_app = response
        .json::<OrgAppDto>()
        .await
        .expect("Should be able to decode OrgAppDto");

    let org_app_id = created_org_app.id.clone();
    assert!(!org_app_id.is_empty(), "Org App ID should not be empty");
    assert_eq!(created_org_app.org_id, org.id, "Org ID should match");
    assert_eq!(created_org_app.app_id, app.id, "App ID should match");

    created_org_app
}

async fn test_create_org_app_not_exists(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    org: &OrgDto,
) {
    info!("test_create_org_app_not_exists");

    let new_org_app = serde_json::json!({
        "app_id": "app_99999999999999999999999999999999",
    });

    let url = format!("{}/orgs/{}/apps", &config.base_url, org.id);
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .json(&new_org_app)
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
        .expect("Should be able to decode ErrorMessageDto");
    assert_eq!(
        error_message.status_code, 400,
        "Error status code should be 400 Bad Request"
    );
}

async fn test_create_org_app_already_exists(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    org: &OrgDto,
    app: &AppDto,
) {
    info!("test_create_org_app_already_exists");

    let new_org_app = serde_json::json!({
        "app_id": app.id,
    });

    let url = format!("{}/orgs/{}/apps", &config.base_url, org.id);
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .json(&new_org_app)
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
        .expect("Should be able to decode ErrorMessageDto");
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

    let new_org_app = serde_json::json!({
        "app_id": app.id,
    });

    let url = format!("{}/orgs/{}/apps", &config.base_url, org.id);
    let response = client
        .post(&url)
        .json(&new_org_app)
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Response should be 401 Unauthorized"
    );

    let error_message = response
        .json::<ErrorMessageDto>()
        .await
        .expect("Should be able to decode ErrorMessageDto");
    assert_eq!(
        error_message.status_code, 401,
        "Error status code should be 401 Unauthorized"
    );
}

async fn test_get_org_app(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    org_app: &OrgAppDto,
) {
    info!("test_get_org_app");

    let url = format!(
        "{}/orgs/{}/apps/{}",
        &config.base_url, org_app.org_id, org_app.app_id
    );
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

    let found_org_app = response
        .json::<OrgAppDto>()
        .await
        .expect("Should be able to decode OrgAppDto");

    assert_eq!(found_org_app.id, org_app.id, "Org App ID should match");
    assert_eq!(found_org_app.org_id, org_app.org_id, "Org ID should match");
    assert_eq!(found_org_app.app_id, org_app.app_id, "App ID should match");
    assert_eq!(
        &found_org_app.app_name, &org_app.app_name,
        "Name should match"
    );
}

async fn test_get_org_app_not_found(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    org: &OrgDto,
) {
    info!("test_get_org_app_not_found");

    let url = format!("{}/orgs/{}/apps/{}", &config.base_url, org.id, 999999);
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

    let error_message = response
        .json::<ErrorMessageDto>()
        .await
        .expect("Should be able to decode ErrorMessageDto");
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

    let error_message = response
        .json::<ErrorMessageDto>()
        .await
        .expect("Should be able to decode ErrorMessageDto");
    assert_eq!(
        error_message.status_code, 401,
        "Error status code should be 401 Unauthorized"
    );
}

async fn test_delete_org_app(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    org_app: &OrgAppDto,
) {
    info!("test_delete_org_app");

    let url = format!(
        "{}/orgs/{}/apps/{}",
        &config.base_url, org_app.org_id, org_app.app_id
    );
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

    let error_message = get_response
        .json::<ErrorMessageDto>()
        .await
        .expect("Should be able to decode ErrorMessageDto");
    assert_eq!(
        error_message.status_code, 404,
        "Error status code should be 404 Not Found"
    );
}

async fn test_delete_org_app_not_found(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    org: &OrgDto,
) {
    info!("test_delete_org_app_not_found");

    let url = format!("{}/orgs/{}/apps/{}", &config.base_url, org.id, 999999);
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

    let error_message = delete_response
        .json::<ErrorMessageDto>()
        .await
        .expect("Should be able to decode ErrorMessageDto");
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

    let error_message = delete_response
        .json::<ErrorMessageDto>()
        .await
        .expect("Should be able to decode ErrorMessageDto");
    assert_eq!(
        error_message.status_code, 401,
        "Error status code should be 401 Unauthorized"
    );
}

async fn delete_test_user(client: &Client, config: &Config, actor: &TestActor, user: &UserDto) {
    info!("delete_test_user");

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

    let error_message = get_response
        .json::<ErrorMessageDto>()
        .await
        .expect("Should be able to decode ErrorMessageDto");
    assert_eq!(
        error_message.status_code, 404,
        "Error status code should be 404 Not Found"
    );
}

async fn delete_test_org(client: &Client, config: &Config, actor: &TestActor, org: &OrgDto) {
    info!("delete_test_org");

    // Delete org owner first
    let url = format!(
        "{}/orgs/{}/members/{}",
        &config.base_url,
        org.id,
        org.owner_id.clone().unwrap()
    );
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

    // Finally, delete the org
    let url = format!("{}/orgs/{}", &config.base_url, org.id);
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

    let error_message = get_response
        .json::<ErrorMessageDto>()
        .await
        .expect("Should be able to decode ErrorMessageDto");
    assert_eq!(
        error_message.status_code, 404,
        "Error status code should be 404 Not Found"
    );
}

async fn delete_test_app(client: &Client, config: &Config, actor: &TestActor, app: &AppDto) {
    info!("delete_test_app");

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

    let error_message = get_response
        .json::<ErrorMessageDto>()
        .await
        .expect("Should be able to decode ErrorMessageDto");
    assert_eq!(
        error_message.status_code, 404,
        "Error status code should be 404 Not Found"
    );
}
