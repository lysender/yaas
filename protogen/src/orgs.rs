use chrono::Utc;
use prost::Message;
use reqwest::{Client, StatusCode};
use tracing::info;

use crate::{TestActor, authenticate_user, config::Config};
use yaas::{
    buffed::dto::{
        ErrorMessageBuf, NewOrgBuf, NewUserWithPasswordBuf, OrgBuf, PaginatedOrgMembershipsBuf,
        PaginatedOrgOwnerSuggestionsBuf, PaginatedOrgsBuf, PaginatedUsersBuf, UpdateOrgBuf,
        UserBuf,
    },
    dto::{CredentialsDto, OrgDto, UserDto},
};

pub async fn run_tests(client: &Client, config: &Config, actor: &TestActor) {
    info!("Running orgs tests");

    // Need a user to own the org
    let owner = create_test_user(client, config, actor).await;

    test_org_owner_suggestions(client, config, actor).await;
    test_org_owner_suggestions_with_exclude(client, config, actor, &owner).await;

    let org = test_create_org(client, config, actor, &owner).await;
    test_create_org_with_superuser_owner(client, config, actor).await;
    test_create_org_unauthenticated(client, config, &owner).await;

    test_orgs_listing(client, config, actor, &org).await;
    test_orgs_listing_unauthenticated(client, config).await;

    test_orgs_listing_non_superuser(client, config, &org, &owner).await;
    test_users_listing_non_superuser(client, config, &owner).await;
    test_org_membership_listing_non_superuser(client, config, &org, &owner).await;

    test_get_org(client, config, actor, &org).await;
    test_get_org_not_found(client, config, actor).await;
    test_get_org_unauthenticated(client, config, &org).await;

    test_update_org_no_changes(client, config, actor, &org).await;
    test_update_org(client, config, actor, &org).await;
    test_update_org_name_only(client, config, actor, &org).await;
    test_update_org_unauthenticated(client, config, &org).await;

    test_delete_org_with_members(client, config, actor, &org).await;
    test_delete_org_cleanup_members(client, config, actor, &org).await;
    test_delete_org_not_found(client, config, actor).await;
    test_delete_org_unauthorized(client, config, &org).await;

    // Cleanup the owner user
    delete_test_user(client, config, actor, &owner).await;
}

async fn test_orgs_listing(client: &Client, config: &Config, actor: &TestActor, org: &OrgDto) {
    info!("test_orgs_listing");

    let url = format!("{}/orgs", &config.base_url);
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

    let listing = PaginatedOrgsBuf::decode(&body_bytes[..])
        .expect("Should be able to decode PaginatedOrgsBuf");

    let meta = listing.meta.unwrap();
    assert!(meta.page == 1, "Page should be 1");
    assert!(meta.per_page == 50, "Per page should be 50");
    assert!(meta.total_records >= 1, "Total records should be >= 1");
    assert!(meta.total_pages >= 1, "Total pages should be >= 1");

    assert!(!listing.data.is_empty(), "There should be at least one org");

    let found = listing.data.iter().find(|o| o.id == org.id);
    assert!(found.is_some(), "Created org should be in the listing");
}

async fn test_orgs_listing_non_superuser(
    client: &Client,
    config: &Config,
    org: &OrgDto,
    owner: &UserDto,
) {
    info!("test_orgs_listing_non_superuser");

    // Authenticate as the owner user
    let actor = authenticate_user(
        client,
        config,
        CredentialsDto {
            email: owner.email.clone(),
            password: "password".to_string(),
        },
    )
    .await;

    let url = format!("{}/orgs", &config.base_url);
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

    let listing = PaginatedOrgsBuf::decode(&body_bytes[..])
        .expect("Should be able to decode PaginatedOrgsBuf");

    let meta = listing.meta.unwrap();
    assert!(meta.page == 1, "Page should be 1");
    assert!(meta.per_page == 50, "Per page should be 50");
    assert!(meta.total_records == 1, "Total records should be == 1");
    assert!(meta.total_pages == 1, "Total pages should be == 1");

    assert!(listing.data.len() == 1, "There should be only one org");

    let found = listing.data.iter().find(|o| o.id == org.id);
    assert!(found.is_some(), "Created org should be in the listing");
}

async fn test_users_listing_non_superuser(client: &Client, config: &Config, owner: &UserDto) {
    info!("test_users_listing_non_superuser");

    // Authenticate as the created user
    let actor = authenticate_user(
        client,
        config,
        CredentialsDto {
            email: owner.email.clone(),
            password: "password".to_string(),
        },
    )
    .await;

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
    assert!(meta.total_records == 1, "Total records should be == 1");
    assert!(meta.total_pages == 1, "Total pages should be == 1");

    assert!(listing.data.len() == 1, "There should be only one user");

    // User must be in the listing
    let found = listing.data.iter().find(|u| u.id == owner.id);
    assert!(found.is_some(), "Created user should be in the listing");
}

async fn test_org_membership_listing_non_superuser(
    client: &Client,
    config: &Config,
    org: &OrgDto,
    owner: &UserDto,
) {
    info!("test_org_membership_listing_non_superuser");

    // Authenticate as the created user
    let actor = authenticate_user(
        client,
        config,
        CredentialsDto {
            email: owner.email.clone(),
            password: "password".to_string(),
        },
    )
    .await;

    let url = format!("{}/user/orgs", &config.base_url);
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

    let listing = PaginatedOrgMembershipsBuf::decode(&body_bytes[..])
        .expect("Should be able to decode PaginatedOrgMembershipsBuf");

    let meta = listing.meta.unwrap();
    assert!(meta.page == 1, "Page should be 1");
    assert!(meta.per_page == 50, "Per page should be 50");
    assert!(meta.total_records == 1, "Total records should be == 1");
    assert!(meta.total_pages == 1, "Total pages should be == 1");

    assert!(
        listing.data.len() == 1,
        "There should be only one org membership"
    );

    // Org must be in the listing
    let found = listing.data.iter().find(|u| u.org_id == org.id);
    let membership = found.expect("Created org membership should be in the listing");
    assert_eq!(membership.org_name, org.name, "Org name should match");
}

async fn test_orgs_listing_unauthenticated(client: &Client, config: &Config) {
    info!("test_orgs_listing_unauthenticated");

    let url = format!("{}/orgs", &config.base_url);
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

async fn test_org_owner_suggestions(client: &Client, config: &Config, actor: &TestActor) {
    info!("test_org_owner_suggestions");

    let url = format!(
        "{}/orgs/owner-suggestions?page=1&per_page=50&keyword=",
        &config.base_url
    );
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

    let listing = PaginatedOrgOwnerSuggestionsBuf::decode(&body_bytes[..])
        .expect("Should be able to decode PaginatedOrgOwnerSuggestionsBuf");

    let meta = listing.meta.unwrap();
    assert!(meta.page == 1, "Page should be 1");
    assert!(meta.per_page == 50, "Per page should be 50");
    assert!(meta.total_records >= 1, "Total records should be >= 1");
    assert!(meta.total_pages >= 1, "Total pages should be >= 1");

    assert!(
        !listing.data.is_empty(),
        "There should be at least one user"
    );

    // Superuser must not be in the list
    let found = listing.data.iter().find(|u| u.id == actor.id);
    assert!(found.is_none(), "Superuser must not be in the suggestions");
}

async fn test_org_owner_suggestions_with_exclude(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    exclude_user: &UserDto,
) {
    info!("test_org_owner_suggestions_with_exclude");

    let url = format!(
        "{}/orgs/owner-suggestions?page=1&per_page=50&keyword=&exclude_id={}",
        &config.base_url, exclude_user.id
    );
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

    let listing = PaginatedOrgOwnerSuggestionsBuf::decode(&body_bytes[..])
        .expect("Should be able to decode PaginatedOrgOwnerSuggestionsBuf");

    let meta = listing.meta.unwrap();
    assert!(meta.page == 1, "Page should be 1");
    assert!(meta.per_page == 50, "Per page should be 50");
    assert!(meta.total_records >= 0, "Total records should be >= 0");
    assert!(meta.total_pages >= 0, "Total pages should be >= 0");

    // Superuser must not be in the list
    let found_superuser = listing.data.iter().find(|u| u.id == actor.id);
    assert!(
        found_superuser.is_none(),
        "Superuser must not be in the suggestions"
    );

    // Excluded user must not be in the list
    let found_excluded = listing.data.iter().find(|u| u.id == exclude_user.id);
    assert!(
        found_excluded.is_none(),
        "Excluded user must not be in the suggestions"
    );
}

async fn create_test_user(client: &Client, config: &Config, actor: &TestActor) -> UserDto {
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

async fn test_create_org(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    owner: &UserDto,
) -> OrgDto {
    info!("test_create_org");

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

async fn test_create_org_with_superuser_owner(client: &Client, config: &Config, actor: &TestActor) {
    info!("test_create_org_with_superuser_owner");

    let random_pad = Utc::now().timestamp_millis();

    let name = format!("Test Org {}", random_pad);

    let new_org = NewOrgBuf {
        name: name.clone(),
        owner_id: actor.id,
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

async fn test_create_org_unauthenticated(client: &Client, config: &Config, owner: &UserDto) {
    info!("test_create_org_unauthenticated");

    let random_pad = Utc::now().timestamp_millis();

    let name = format!("Test Org {}", random_pad);

    let new_org = NewOrgBuf {
        name: name.clone(),
        owner_id: owner.id,
    };

    let url = format!("{}/orgs", &config.base_url);
    let response = client
        .post(&url)
        .body(new_org.encode_to_vec())
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

async fn test_get_org(client: &Client, config: &Config, actor: &TestActor, org: &OrgDto) {
    info!("test_get_org");

    let url = format!("{}/orgs/{}", &config.base_url, org.id);
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

    let found_org = OrgBuf::decode(&body_bytes[..]).expect("Should be able to decode OrgBuf");
    assert_eq!(found_org.id, org.id, "Org ID should match");
    assert_eq!(&found_org.name, &org.name, "Name should match");
}

async fn test_get_org_not_found(client: &Client, config: &Config, actor: &TestActor) {
    info!("test_get_org_not_found");

    let url = format!("{}/orgs/{}", &config.base_url, 999999);
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

async fn test_get_org_unauthenticated(client: &Client, config: &Config, user: &OrgDto) {
    info!("test_get_user_unauthenticated");

    let url = format!("{}/orgs/{}", &config.base_url, user.id);
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

async fn test_update_org_no_changes(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    org: &OrgDto,
) {
    info!("test_update_org_no_changes");

    let data = UpdateOrgBuf {
        name: None,
        status: None,
        owner_id: None,
    };

    let url = format!("{}/orgs/{}", &config.base_url, org.id);
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

    let updated_org = OrgBuf::decode(&body_bytes[..]).expect("Should be able to decode OrgBuf");
    assert_eq!(&updated_org.name, &org.name, "Name should be the same");
    assert_eq!(
        &updated_org.status, &org.status,
        "Status should be the same"
    );
    assert_eq!(
        updated_org.owner_id, org.owner_id,
        "Owner ID should be the same"
    );
}

async fn test_update_org(client: &Client, config: &Config, actor: &TestActor, org: &OrgDto) {
    info!("test_update_org");

    let updated_name = format!("{} v2", org.name);
    let updated_status = "inactive".to_string();

    let data = UpdateOrgBuf {
        name: Some(updated_name.clone()),
        status: Some(updated_status.clone()),
        owner_id: None,
    };

    let url = format!("{}/orgs/{}", &config.base_url, org.id);
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

    let updated_org = OrgBuf::decode(&body_bytes[..]).expect("Should be able to decode OrgBuf");
    assert_eq!(&updated_org.name, &updated_name, "Name should be updated");
    assert_eq!(
        &updated_org.status, &updated_status,
        "Status should be updated"
    );
}

async fn test_update_org_name_only(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    user: &OrgDto,
) {
    info!("test_update_org_name_only");

    let data = UpdateOrgBuf {
        name: Some(user.name.clone()),
        status: None,
        owner_id: None,
    };

    let url = format!("{}/orgs/{}", &config.base_url, user.id);
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

    let updated_org = OrgBuf::decode(&body_bytes[..]).expect("Should be able to decode OrgBuf");
    assert_eq!(
        &updated_org.name, &user.name,
        "Name should be reverted back to original"
    );
    assert_eq!(
        &updated_org.status, "inactive",
        "Status should be still be the same"
    );
}

async fn test_update_org_unauthenticated(client: &Client, config: &Config, user: &OrgDto) {
    info!("test_update_org_unauthenticated");

    let data = UpdateOrgBuf {
        name: None,
        status: None,
        owner_id: None,
    };

    let url = format!("{}/orgs/{}", &config.base_url, user.id);
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

async fn test_delete_org_cleanup_members(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    org: &OrgDto,
) {
    info!("test_delete_org_cleanup_members");

    // Remove owner member first
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

    // Delete the actual org
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

async fn test_delete_org_with_members(
    client: &Client,
    config: &Config,
    actor: &TestActor,
    org: &OrgDto,
) {
    info!("test_delete_org_with_members");

    let url = format!("{}/orgs/{}", &config.base_url, org.id);
    let delete_response = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", &actor.token))
        .send()
        .await
        .expect("Should be able to send request");

    assert_eq!(
        delete_response.status(),
        StatusCode::FORBIDDEN,
        "Response should be 403 Forbidden"
    );

    let body_bytes = delete_response
        .bytes()
        .await
        .expect("Should be able to read response body");

    let error_message =
        ErrorMessageBuf::decode(&body_bytes[..]).expect("Should be able to decode ErrorMessageBuf");
    assert_eq!(
        error_message.status_code, 403,
        "Error status code should be 403 Forbidden"
    );
}

async fn test_delete_org_not_found(client: &Client, config: &Config, actor: &TestActor) {
    info!("test_delete_user_not_found");

    let url = format!("{}/orgs/{}", &config.base_url, 999999);
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

async fn test_delete_org_unauthorized(client: &Client, config: &Config, org: &OrgDto) {
    info!("test_delete_user_unauthorized");

    let url = format!("{}/orgs/{}", &config.base_url, org.id);
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

async fn delete_test_user(client: &Client, config: &Config, actor: &TestActor, user: &UserDto) {
    info!("delete_test_org");

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
