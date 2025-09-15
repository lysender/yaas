use chrono::Utc;
use prost::Message;
use reqwest::{Client, StatusCode};
use tracing::info;

use crate::config::Config;
use yaas::{
    buffed::dto::{
        ErrorMessageBuf, NewOrgBuf, NewOrgMemberBuf, NewUserWithPasswordBuf, OrgBuf, OrgMemberBuf,
        PaginatedOrgMembersBuf, UpdateOrgMemberBuf, UserBuf,
    },
    dto::{OrgDto, OrgMemberDto, UserDto},
    role::{Role, to_buffed_roles},
};

pub async fn run_tests(client: &Client, config: &Config, token: &str) {
    info!("Running org members tests");

    // Need a user to own the org
    let admin_user = create_test_user(client, config, token).await;

    // Need an org to work with
    let org = create_test_org(client, config, token, &admin_user).await;

    // Create the org admin user
    let org_admin = create_test_org_member(
        client,
        config,
        token,
        &org,
        &admin_user,
        &vec![Role::OrgAdmin],
    )
    .await;

    test_org_members_listing(client, config, token, &org).await;
    test_org_members_listing_unauthenticated(client, config, &org).await;

    let member_user = create_test_user(client, config, token).await;
    let another_user = create_test_user(client, config, token).await;

    let org_member = create_test_org_member(
        client,
        config,
        token,
        &org,
        &member_user,
        &vec![Role::OrgEditor],
    )
    .await;
    test_create_org_member_unauthenticated(client, config, &org, &another_user).await;

    test_get_org_member(client, config, token, &org, &org_admin).await;
    test_get_org_member(client, config, token, &org, &org_member).await;
    test_get_org_member_not_found(client, config, token, &org).await;
    test_get_org_member_unauthenticated(client, config, &org, &org_admin).await;

    test_update_org_member_no_changes(client, config, token, &org, &org_member).await;
    test_update_org_member(client, config, token, &org, &org_member).await;
    test_update_org_member_status_only(client, config, token, &org, &org_member).await;
    test_update_org_member_unauthenticated(client, config, &org, &org_member).await;

    test_delete_org_member_not_found(client, config, token, &org).await;
    test_delete_org_member_unauthorized(client, config, &org, &org_member).await;
    test_delete_org_member(client, config, token, &org, &org_admin).await;
    test_delete_org_member(client, config, token, &org, &org_member).await;

    // Cleanup created resources
    delete_test_org(client, config, token, &org).await;
    delete_test_user(client, config, token, &admin_user).await;
    delete_test_user(client, config, token, &member_user).await;
    delete_test_user(client, config, token, &another_user).await;
}

async fn test_org_members_listing(client: &Client, config: &Config, token: &str, org: &OrgDto) {
    info!("test_org_members_listing");

    let url = format!("{}/orgs/{}/members", &config.base_url, org.id);
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

    let listing = PaginatedOrgMembersBuf::decode(&body_bytes[..])
        .expect("Should be able to decode PaginatedOrgsBuf");

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

async fn test_org_members_listing_unauthenticated(client: &Client, config: &Config, org: &OrgDto) {
    info!("test_org_members_listing_unauthenticated");

    let url = format!("{}/orgs/{}/members", &config.base_url, org.id);
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
    assert_eq!(created_org.owner_id, owner.id, "Owner ID should match");

    let dto: OrgDto = created_org.into();
    dto
}

async fn create_test_org_member(
    client: &Client,
    config: &Config,
    token: &str,
    org: &OrgDto,
    user: &UserDto,
    roles: &Vec<Role>,
) -> OrgMemberDto {
    info!("create_test_org_member");

    let new_member = NewOrgMemberBuf {
        user_id: user.id,
        roles: to_buffed_roles(roles),
        status: "active".to_string(),
    };

    let url = format!("{}/orgs/{}/members", &config.base_url, org.id);
    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", token))
        .body(new_member.encode_to_vec())
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

    let created_member =
        OrgMemberBuf::decode(&body_bytes[..]).expect("Should be able to decode OrgMemberBuf");
    let member_id = created_member.id;
    assert!(member_id > 0, "User ID should be greater than 0");
    assert_eq!(created_member.org_id, org.id, "Org ID should match");
    assert_eq!(created_member.user_id, user.id, "User ID should match");

    let dto: OrgMemberDto = created_member
        .try_into()
        .expect("Should be able to convert to OrgMemberDto");
    dto
}

async fn test_create_org_member_unauthenticated(
    client: &Client,
    config: &Config,
    org: &OrgDto,
    user: &UserDto,
) {
    info!("test_create_org_member_unauthenticated");

    let new_member = NewOrgMemberBuf {
        user_id: user.id,
        roles: to_buffed_roles(&vec![Role::OrgAdmin]),
        status: "active".to_string(),
    };

    let url = format!("{}/orgs/{}/members", &config.base_url, org.id);
    let response = client
        .post(&url)
        .body(new_member.encode_to_vec())
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

async fn test_get_org_member(
    client: &Client,
    config: &Config,
    token: &str,
    org: &OrgDto,
    member: &OrgMemberDto,
) {
    info!("test_get_org_member");

    let url = format!("{}/orgs/{}/members/{}", &config.base_url, org.id, member.id);
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

    let found_member =
        OrgMemberBuf::decode(&body_bytes[..]).expect("Should be able to decode OrgMemberBuf");
    assert_eq!(found_member.id, member.id, "Org Member ID should match");
    assert_eq!(found_member.org_id, member.org_id, "Org ID should match");
    assert_eq!(found_member.user_id, member.user_id, "User ID should match");
    assert_eq!(&found_member.name, &None, "Name should match");
}

async fn test_get_org_member_not_found(
    client: &Client,
    config: &Config,
    token: &str,
    org: &OrgDto,
) {
    info!("test_get_org_member_not_found");

    let url = format!("{}/orgs/{}/members/{}", &config.base_url, org.id, 999999);
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

async fn test_get_org_member_unauthenticated(
    client: &Client,
    config: &Config,
    org: &OrgDto,
    member: &OrgMemberDto,
) {
    info!("test_get_org_member_unauthenticated");

    let url = format!("{}/orgs/{}/members/{}", &config.base_url, org.id, member.id);
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

async fn test_update_org_member_no_changes(
    client: &Client,
    config: &Config,
    token: &str,
    org: &OrgDto,
    member: &OrgMemberDto,
) {
    info!("test_update_org_member_no_changes");

    // Empty roles is interpreted as no changes to roles
    let data = UpdateOrgMemberBuf {
        roles: vec![],
        status: None,
    };

    let url = format!("{}/orgs/{}/members/{}", &config.base_url, org.id, member.id);
    let response = client
        .patch(&url)
        .header("Authorization", format!("Bearer {}", token))
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

    let updated_member =
        OrgMemberBuf::decode(&body_bytes[..]).expect("Should be able to decode OrgMemberBuf");
    assert_eq!(
        &updated_member.status, &member.status,
        "Status should be the same"
    );
    assert_eq!(
        &updated_member.roles,
        &to_buffed_roles(&member.roles),
        "Roles should be the same"
    );
}

async fn test_update_org_member(
    client: &Client,
    config: &Config,
    token: &str,
    org: &OrgDto,
    member: &OrgMemberDto,
) {
    info!("test_update_org_member");

    let data = UpdateOrgMemberBuf {
        roles: to_buffed_roles(&vec![Role::OrgViewer]),
        status: Some("inactive".to_string()),
    };

    let url = format!("{}/orgs/{}/members/{}", &config.base_url, org.id, member.id);
    let response = client
        .patch(&url)
        .header("Authorization", format!("Bearer {}", token))
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

    let updated_member =
        OrgMemberBuf::decode(&body_bytes[..]).expect("Should be able to decode OrgMemberBuf");
    assert_eq!(&updated_member.roles, &data.roles, "Name should be updated");
    assert_eq!(
        &updated_member.status,
        &data.status.unwrap(),
        "Status should be updated"
    );
}

async fn test_update_org_member_status_only(
    client: &Client,
    config: &Config,
    token: &str,
    org: &OrgDto,
    member: &OrgMemberDto,
) {
    info!("test_update_org_member_status_only");

    let data = UpdateOrgMemberBuf {
        roles: vec![],
        status: Some("active".to_string()),
    };

    let url = format!("{}/orgs/{}/members/{}", &config.base_url, org.id, member.id);
    let response = client
        .patch(&url)
        .header("Authorization", format!("Bearer {}", token))
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

    let updated_member =
        OrgMemberBuf::decode(&body_bytes[..]).expect("Should be able to decode OrgMemberBuf");
    assert_eq!(&updated_member.status, "active", "Status should be active");
    assert_eq!(
        &updated_member.roles,
        &to_buffed_roles(&vec![Role::OrgViewer]),
        "Roles should be still be the same"
    );
}

async fn test_update_org_member_unauthenticated(
    client: &Client,
    config: &Config,
    org: &OrgDto,
    member: &OrgMemberDto,
) {
    info!("test_update_org_member_unauthenticated");

    let data = UpdateOrgMemberBuf {
        roles: vec![],
        status: None,
    };

    let url = format!("{}/orgs/{}/members/{}", &config.base_url, org.id, member.id);
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

async fn test_delete_org_member(
    client: &Client,
    config: &Config,
    token: &str,
    org: &OrgDto,
    member: &OrgMemberDto,
) {
    info!("test_delete_org_member");

    let url = format!("{}/orgs/{}/members/{}", &config.base_url, org.id, member.id);
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

async fn test_delete_org_member_not_found(
    client: &Client,
    config: &Config,
    token: &str,
    org: &OrgDto,
) {
    info!("test_delete_org_member_not_found");

    let url = format!("{}/orgs/{}/members/{}", &config.base_url, org.id, 999999);
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

async fn test_delete_org_member_unauthorized(
    client: &Client,
    config: &Config,
    org: &OrgDto,
    member: &OrgMemberDto,
) {
    info!("test_delete_org_member_unauthorized");

    let url = format!("{}/orgs/{}/members/{}", &config.base_url, org.id, member.id);
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
    info!("delete_test_org");

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
