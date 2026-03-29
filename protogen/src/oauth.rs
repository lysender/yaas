use chrono::Utc;
use reqwest::{Client, StatusCode};
use tracing::info;

use crate::{TestActor, authenticate_user, config::Config};
use yaas::dto::{
    AppDto, CredentialsDto, ErrorMessageDto, NewAppDto, NewOrgAppDto, NewOrgDto,
    NewUserWithPasswordDto, OauthAuthorizationCodeDto, OauthAuthorizeDto, OauthTokenRequestDto,
    OauthTokenResponseDto, OrgAppDto, OrgDto, UserDto,
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

    test_oauth_token_success(client, config, &test_user, &test_app).await;
    test_oauth_token_invalid_secret(client, config, &test_user, &test_app).await;
    test_oauth_token_mismatched_state(client, config, &test_user, &test_app).await;
    test_oauth_token_mismatched_redirect_uri(client, config, &test_user, &test_app).await;

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
    let payload = OauthAuthorizeDto {
        client_id: app.client_id.clone(),
        redirect_uri: app.redirect_uri.clone(),
        scope: "auth".to_string(),
        state: format!("state-{}", Utc::now().timestamp_millis()),
    };

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .json(&payload)
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Response should be 200 OK"
    );

    let auth_code = response
        .json::<OauthAuthorizationCodeDto>()
        .await
        .expect("Should be able to decode OauthAuthorizationCodeDto");

    assert!(!auth_code.code.is_empty(), "Auth code should not be empty");
    assert_eq!(auth_code.state, payload.state, "State should match request");
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
    let payload = OauthAuthorizeDto {
        client_id: "00000000-0000-0000-0000-000000000000".to_string(),
        redirect_uri: app.redirect_uri.clone(),
        scope: "auth oauth".to_string(),
        state: "invalid-client".to_string(),
    };

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .json(&payload)
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
    let payload = OauthAuthorizeDto {
        client_id: app.client_id.clone(),
        redirect_uri: app.redirect_uri.clone(),
        scope: "auth oauth".to_string(),
        state: "unlinked-app".to_string(),
    };

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .json(&payload)
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

async fn test_oauth_authorize_missing_token(client: &Client, config: &Config, app: &AppDto) {
    info!("test_oauth_authorize_missing_token");

    let url = format!("{}/oauth/authorize", &config.base_url);
    let payload = OauthAuthorizeDto {
        client_id: app.client_id.clone(),
        redirect_uri: app.redirect_uri.clone(),
        scope: "org.read".to_string(),
        state: "missing-token".to_string(),
    };

    let response = client
        .post(&url)
        .json(&payload)
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

async fn test_oauth_token_success(client: &Client, config: &Config, user: &UserDto, app: &AppDto) {
    info!("test_oauth_token_success");

    let actor = authenticate_user(
        client,
        config,
        CredentialsDto {
            email: user.email.clone(),
            password: "password".to_string(),
        },
    )
    .await;

    let auth_result = create_oauth_authorization_code(client, config, &actor, app).await;

    let url = format!("{}/oauth/token", &config.base_url);
    let payload = OauthTokenRequestDto {
        client_id: app.client_id.clone(),
        client_secret: app.client_secret.clone(),
        code: auth_result.auth_code.code,
        state: auth_result.state,
        redirect_uri: auth_result.redirect_uri,
    };

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .json(&payload)
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Response should be 200 OK"
    );

    let token_response = response
        .json::<OauthTokenResponseDto>()
        .await
        .expect("Should be able to decode OauthTokenResponseDto");

    assert!(
        !token_response.access_token.is_empty(),
        "Access token should not be empty"
    );
    assert_eq!(
        token_response.scope, auth_result.scope,
        "Scope should match"
    );
    assert_eq!(token_response.token_type, "app", "Token type should be app");

    let user_url = format!("{}/user", &config.base_url);
    let user_response = client
        .get(&user_url)
        .header(
            "Authorization",
            format!("Bearer {}", token_response.access_token),
        )
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        user_response.status(),
        StatusCode::OK,
        "Response should be 200 OK"
    );

    let current_user = user_response
        .json::<UserDto>()
        .await
        .expect("Should be able to decode UserDto");

    assert_eq!(current_user.id, actor.id, "User ID should match");
}

async fn test_oauth_token_invalid_secret(
    client: &Client,
    config: &Config,
    user: &UserDto,
    app: &AppDto,
) {
    info!("test_oauth_token_invalid_secret");

    let actor = authenticate_user(
        client,
        config,
        CredentialsDto {
            email: user.email.clone(),
            password: "password".to_string(),
        },
    )
    .await;

    let auth_result = create_oauth_authorization_code(client, config, &actor, app).await;

    let url = format!("{}/oauth/token", &config.base_url);
    let payload = OauthTokenRequestDto {
        client_id: app.client_id.clone(),
        client_secret: "00000000-0000-0000-0000-000000000000".to_string(),
        code: auth_result.auth_code.code,
        state: auth_result.state,
        redirect_uri: auth_result.redirect_uri,
    };

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .json(&payload)
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Response should be 401 Unauthorized"
    );
}

async fn test_oauth_token_mismatched_state(
    client: &Client,
    config: &Config,
    user: &UserDto,
    app: &AppDto,
) {
    info!("test_oauth_token_mismatched_state");

    let actor = authenticate_user(
        client,
        config,
        CredentialsDto {
            email: user.email.clone(),
            password: "password".to_string(),
        },
    )
    .await;

    let auth_result = create_oauth_authorization_code(client, config, &actor, app).await;

    let url = format!("{}/oauth/token", &config.base_url);
    let payload = OauthTokenRequestDto {
        client_id: app.client_id.clone(),
        client_secret: app.client_secret.clone(),
        code: auth_result.auth_code.code,
        state: "mismatched-state".to_string(),
        redirect_uri: auth_result.redirect_uri,
    };

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .json(&payload)
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Response should be 401 Unauthorized"
    );
}

async fn test_oauth_token_mismatched_redirect_uri(
    client: &Client,
    config: &Config,
    user: &UserDto,
    app: &AppDto,
) {
    info!("test_oauth_token_mismatched_redirect_uri");

    let actor = authenticate_user(
        client,
        config,
        CredentialsDto {
            email: user.email.clone(),
            password: "password".to_string(),
        },
    )
    .await;

    let auth_result = create_oauth_authorization_code(client, config, &actor, app).await;

    let url = format!("{}/oauth/token", &config.base_url);
    let payload = OauthTokenRequestDto {
        client_id: app.client_id.clone(),
        client_secret: app.client_secret.clone(),
        code: auth_result.auth_code.code,
        state: auth_result.state,
        redirect_uri: "https://example.com/mismatch".to_string(),
    };

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .json(&payload)
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        response.status(),
        StatusCode::UNAUTHORIZED,
        "Response should be 401 Unauthorized"
    );
}

struct AuthorizationCodeResult {
    auth_code: OauthAuthorizationCodeDto,
    state: String,
    scope: String,
    redirect_uri: String,
}

async fn create_oauth_authorization_code(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    app: &AppDto,
) -> AuthorizationCodeResult {
    let url = format!("{}/oauth/authorize", &config.base_url);
    let payload = OauthAuthorizeDto {
        client_id: app.client_id.clone(),
        redirect_uri: app.redirect_uri.clone(),
        scope: "auth".to_string(),
        state: format!("state-{}", Utc::now().timestamp_millis()),
    };

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .json(&payload)
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        response.status(),
        StatusCode::OK,
        "Response should be 200 OK"
    );

    let auth_code_dto = response
        .json::<OauthAuthorizationCodeDto>()
        .await
        .expect("Should be able to decode OauthAuthorizationCodeDto");

    AuthorizationCodeResult {
        auth_code: auth_code_dto,
        state: payload.state,
        scope: payload.scope,
        redirect_uri: app.redirect_uri.clone(),
    }
}

async fn create_test_user(client: &Client, config: &Config, actor: &TestActor) -> UserDto {
    info!("create_test_user");

    let random_pad = Utc::now().timestamp_millis();

    let email = format!("testuser.{}@example.com", random_pad);
    let name = format!("Test User {}", random_pad);
    let password = "password".to_string();

    let new_user = NewUserWithPasswordDto {
        email: email.clone(),
        name: name.clone(),
        password,
    };

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

    let new_org = NewOrgDto {
        name: name.clone(),
        owner_id: owner.id.clone(),
    };

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

    let new_app = NewAppDto {
        name: name.clone(),
        redirect_uri: "https://example.com/callback".to_string(),
    };

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

    let new_org_app = NewOrgAppDto {
        app_id: app.id.clone(),
    };

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
