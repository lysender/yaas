use chrono::Utc;
use prost::Message;
use reqwest::{Client, StatusCode};
use tracing::info;

use crate::{TestActor, authenticate_user, config::Config};
use yaas::{
    buffed::dto::{
        AppBuf, ErrorMessageBuf, NewAppBuf, NewOrgAppBuf, NewOrgBuf, NewUserWithPasswordBuf,
        OauthAuthorizationCodeBuf, OauthAuthorizeBuf, OrgAppBuf, OrgBuf, UserBuf,
    },
    dto::{AppDto, CredentialsDto, OrgAppDto, OrgDto, UserDto},
};

pub async fn run_tests(client: &Client, config: &Config, actor: &TestActor) {
    info!("Running oauth tests");

    let test_user = create_test_user(client, config, actor).await;
    let test_org = create_test_org(client, config, actor, &test_user).await;
    let test_app = create_test_app(client, config, actor).await;
    let test_org_app = create_test_org_app(client, config, actor, &test_org, &test_app).await;
    let unlinked_app = create_test_app(client, config, actor).await;

    test_oauth_authorize_success(client, config, &test_user, &test_app).await;
    test_oauth_authorize_invalid_client(client, config, &test_user, &test_app).await;
    test_oauth_authorize_unlinked_app(client, config, &test_user, &unlinked_app).await;
    test_oauth_authorize_missing_token(client, config, &test_app).await;

    delete_test_org_app(client, config, actor, &test_org_app).await;
    delete_test_org(client, config, actor, &test_org).await;
    delete_test_user(client, config, actor, &test_user).await;
    delete_test_app(client, config, actor, &test_app).await;
    delete_test_app(client, config, actor, &unlinked_app).await;
}

async fn test_oauth_authorize_success(
    client: &Client,
    config: &Config,
    user: &UserDto,
    app: &AppDto,
) {
    info!("test_oauth_authorize_success");

    let actor = authenticate_user(
        client,
        config,
        CredentialsDto {
            email: user.email.clone(),
            password: "password".to_string(),
        },
    )
    .await;

    let url = format!("{}/oauth/authorize", &config.base_url);
    let payload = OauthAuthorizeBuf {
        client_id: app.client_id.clone(),
        redirect_uri: app.redirect_uri.clone(),
        scope: "org.read".to_string(),
        state: format!("state-{}", Utc::now().timestamp_millis()),
    };

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .body(payload.encode_to_vec())
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

    let auth_code = OauthAuthorizationCodeBuf::decode(&body_bytes[..])
        .expect("Should be able to decode OauthAuthorizationCodeBuf");

    assert!(!auth_code.code.is_empty(), "Auth code should not be empty");
    assert_eq!(
        auth_code.state, payload.state,
        "State should match request"
    );
}

async fn test_oauth_authorize_invalid_client(
    client: &Client,
    config: &Config,
    user: &UserDto,
    app: &AppDto,
) {
    info!("test_oauth_authorize_invalid_client");

    let actor = authenticate_user(
        client,
        config,
        CredentialsDto {
            email: user.email.clone(),
            password: "password".to_string(),
        },
    )
    .await;

    let url = format!("{}/oauth/authorize", &config.base_url);
    let payload = OauthAuthorizeBuf {
        client_id: "00000000-0000-0000-0000-000000000000".to_string(),
        redirect_uri: app.redirect_uri.clone(),
        scope: "org.read".to_string(),
        state: "invalid-client".to_string(),
    };

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .body(payload.encode_to_vec())
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

async fn test_oauth_authorize_unlinked_app(
    client: &Client,
    config: &Config,
    user: &UserDto,
    app: &AppDto,
) {
    info!("test_oauth_authorize_unlinked_app");

    let actor = authenticate_user(
        client,
        config,
        CredentialsDto {
            email: user.email.clone(),
            password: "password".to_string(),
        },
    )
    .await;

    let url = format!("{}/oauth/authorize", &config.base_url);
    let payload = OauthAuthorizeBuf {
        client_id: app.client_id.clone(),
        redirect_uri: app.redirect_uri.clone(),
        scope: "org.read".to_string(),
        state: "unlinked-app".to_string(),
    };

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .body(payload.encode_to_vec())
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

async fn test_oauth_authorize_missing_token(client: &Client, config: &Config, app: &AppDto) {
    info!("test_oauth_authorize_missing_token");

    let url = format!("{}/oauth/authorize", &config.base_url);
    let payload = OauthAuthorizeBuf {
        client_id: app.client_id.clone(),
        redirect_uri: app.redirect_uri.clone(),
        scope: "org.read".to_string(),
        state: "missing-token".to_string(),
    };

    let response = client
        .post(&url)
        .body(payload.encode_to_vec())
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

async fn create_test_user(client: &Client, config: &Config, actor: &TestActor) -> UserDto {
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

    let created_user = UserBuf::decode(&body_bytes[..]).expect("Should be able to decode UserBuf");
    let user_id = created_user.id;
    assert!(user_id > 0, "User ID should be greater than 0");
    assert_eq!(created_user.email, email, "Email should match");
    assert_eq!(created_user.name, name, "Name should match");

    let dto: UserDto = created_user.into();
    dto
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

    let new_org = NewOrgBuf {
        name: name.clone(),
        owner_id: owner.id,
    };

    let url = format!("{}/orgs", &config.base_url);
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
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

async fn create_test_app(client: &Client, config: &Config, actor: &TestActor) -> AppDto {
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

async fn create_test_org_app(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    org: &OrgDto,
    app: &AppDto,
) -> OrgAppDto {
    info!("create_test_org_app");

    let new_org_app = NewOrgAppBuf { app_id: app.id };

    let url = format!("{}/orgs/{}/apps", &config.base_url, org.id);
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
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

async fn delete_test_org_app(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    org_app: &OrgAppDto,
) {
    info!("delete_test_org_app");

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
}

async fn delete_test_org(client: &Client, config: &Config, actor: &TestActor, org: &OrgDto) {
    info!("delete_test_org");

    let url = format!(
        "{}/orgs/{}/members/{}",
        &config.base_url,
        org.id,
        org.owner_id.unwrap()
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
}
