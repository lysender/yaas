use serde::{Deserialize, Serialize};
use snafu::ensure;

use crate::dto::{ListOrgAppsParamsDto, ListOrgMembersParamsDto, Paginated};
use crate::dto::{
    ListOrgOwnerSuggestionsParamsDto, ListOrgsParamsDto, NewOrgDto, OrgDto, OrgOwnerSuggestionDto,
    UpdateOrgDto,
};
use crate::error::{CsrfTokenSnafu, ForbiddenSnafu, ValidationSnafu};
use crate::run::AppState;
use crate::services::token::verify_csrf_token;
use crate::{Error, Result};

#[derive(Clone, Deserialize, Serialize)]
pub struct NewOrgFormData {
    pub token: String,
    pub name: String,
    pub owner_id: String,
    pub owner_email: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UpdateOrgFormData {
    pub token: String,
    pub name: String,
    pub active: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UpdateOrgOwnerFormData {
    pub token: String,
    pub owner_id: String,
    pub owner_email: String,
}

#[derive(Clone, Deserialize, Serialize)]
pub struct SelectOrgOwnerParams {
    pub owner_id: String,
    pub owner_email: String,
}

pub async fn list_orgs_svc(
    state: &AppState,
    params: ListOrgsParamsDto,
) -> Result<Paginated<OrgDto>> {
    state.db.orgs.list(params).await
}

pub async fn list_org_owner_suggestions_svc(
    state: &AppState,
    params: ListOrgOwnerSuggestionsParamsDto,
) -> Result<Paginated<OrgOwnerSuggestionDto>> {
    state.db.orgs.list_owner_suggestions(params).await
}

pub async fn create_org_svc(state: &AppState, data: NewOrgDto) -> Result<OrgDto> {
    let owner_id = data.owner_id.clone();

    // Owner must exists
    let owner = state.db.users.get(owner_id.clone()).await?;

    ensure!(
        owner.is_some(),
        ValidationSnafu {
            msg: "Owner does not exists".to_string()
        }
    );

    // Owner must not be a superuser
    let superuser = state.db.superusers.get(owner_id).await?;

    ensure!(
        superuser.is_none(),
        ValidationSnafu {
            msg: "Owner cannot be a superuser".to_string()
        }
    );

    state.db.orgs.create(data).await
}

pub async fn create_org_web_svc(state: &AppState, form: NewOrgFormData) -> Result<OrgDto> {
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == "new_org", CsrfTokenSnafu);

    create_org_svc(
        state,
        NewOrgDto {
            name: form.name,
            owner_id: form.owner_id,
        },
    )
    .await
}

pub async fn get_org_svc(state: &AppState, id: &str) -> Result<Option<OrgDto>> {
    state.db.orgs.get(id.to_string()).await
}

pub async fn update_org_svc(state: &AppState, id: &str, data: UpdateOrgDto) -> Result<bool> {
    // Owner must exists and must be a member of the org
    if let Some(owner_id) = data.owner_id.clone() {
        // User must exists
        let owner = state.db.users.get(owner_id.clone()).await?;

        ensure!(
            owner.is_some(),
            ValidationSnafu {
                msg: "Owner does not exists".to_string()
            }
        );

        // Owner must be an existing member of the org
        let member = state
            .db
            .org_members
            .find_member(id.to_string(), owner_id.clone())
            .await?;

        ensure!(
            member.is_some(),
            ValidationSnafu {
                msg: "Owner must be a member of the org".to_string()
            }
        );

        // Owner must not be a superuser
        let superuser = state.db.superusers.get(owner_id).await?;

        ensure!(
            superuser.is_none(),
            ValidationSnafu {
                msg: "Owner cannot be a superuser".to_string()
            }
        );
    }

    state.db.orgs.update(id.to_string(), data).await
}

pub async fn update_org_web_svc(
    state: &AppState,
    org_id: &str,
    form: UpdateOrgFormData,
) -> Result<OrgDto> {
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == org_id, CsrfTokenSnafu);

    let data = UpdateOrgDto {
        name: Some(form.name),
        owner_id: None,
        status: match form.active {
            Some(_) => Some("active".to_string()),
            None => Some("inactive".to_string()),
        },
    };

    update_org_svc(state, org_id, data).await?;

    // Fetch the updated org to return
    let Some(updated_org) = get_org_svc(state, org_id).await? else {
        return Err(Error::OrgNotFound);
    };

    Ok(updated_org)
}

pub async fn update_org_owner_web_svc(
    state: &AppState,
    org_id: &str,
    form: UpdateOrgOwnerFormData,
) -> Result<OrgDto> {
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == org_id, CsrfTokenSnafu);

    let body = UpdateOrgDto {
        name: None,
        owner_id: Some(form.owner_id),
        status: None,
    };

    update_org_svc(state, org_id, body).await?;

    // Fetch the updated org to return
    let Some(updated_org) = get_org_svc(state, org_id).await? else {
        return Err(Error::OrgNotFound);
    };

    Ok(updated_org)
}

pub async fn delete_org_svc(state: &AppState, id: &str) -> Result<bool> {
    // Ensure no members under the org
    let member_count = state
        .db
        .org_members
        .listing_count(id.to_string(), ListOrgMembersParamsDto::default())
        .await?;

    ensure!(
        member_count == 0,
        ForbiddenSnafu {
            msg: "Cannot delete org with existing members".to_string()
        }
    );

    // Ensure no apps under the org
    let app_count = state
        .db
        .org_apps
        .listing_count(id.to_string(), ListOrgAppsParamsDto::default())
        .await?;

    ensure!(
        app_count == 0,
        ForbiddenSnafu {
            msg: "Cannot delete org with existing apps".to_string()
        }
    );

    state.db.orgs.delete(id.to_string()).await
}

pub async fn delete_org_web_svc(state: &AppState, org_id: &str, csrf_token: &str) -> Result<()> {
    let csrf_result = verify_csrf_token(csrf_token, &state.config.jwt_secret)?;
    ensure!(csrf_result == org_id, CsrfTokenSnafu);

    delete_org_svc(state, org_id).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::dto::NewOrgAppDto;
    use crate::services::org_apps::create_org_app_svc;
    use crate::services::token::create_csrf_token_svc;
    use crate::test::TestCtx;
    use crate::utils::{IdPrefix, generate_id};

    use super::{
        NewOrgFormData, UpdateOrgFormData, UpdateOrgOwnerFormData, create_org_web_svc,
        delete_org_web_svc, get_org_svc, update_org_owner_web_svc, update_org_web_svc,
    };

    #[tokio::test]
    async fn create_org_web_svc_creates_org_and_get_returns_it() {
        let ctx = TestCtx::new("orgs_create_web").await.expect("test ctx");
        let owner = ctx
            .seed_user_with_password("Org Owner", "org.owner@example.com", "password123")
            .await
            .expect("owner user");

        let csrf = create_csrf_token_svc("new_org", &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");

        let created = create_org_web_svc(
            &ctx.state,
            NewOrgFormData {
                token: csrf,
                name: "Platform Org".to_string(),
                owner_id: owner.id.clone(),
                owner_email: owner.email,
            },
        )
        .await
        .expect("org should be created");

        let fetched = get_org_svc(&ctx.state, &created.id)
            .await
            .expect("query should pass")
            .expect("org should exist");

        assert_eq!(fetched.id, created.id);
        assert_eq!(fetched.name, "Platform Org");
        assert_eq!(fetched.owner_id, Some(owner.id));
    }

    #[tokio::test]
    async fn create_org_web_svc_rejects_invalid_csrf_token() {
        let ctx = TestCtx::new("orgs_create_invalid_csrf")
            .await
            .expect("test ctx");
        let owner = ctx
            .seed_user_with_password("Org Owner", "org.owner.csrf@example.com", "password123")
            .await
            .expect("owner user");

        let result = create_org_web_svc(
            &ctx.state,
            NewOrgFormData {
                token: "invalid.token".to_string(),
                name: "Platform Org".to_string(),
                owner_id: owner.id,
                owner_email: owner.email,
            },
        )
        .await;

        assert!(result.is_err(), "invalid csrf should fail");
    }

    #[tokio::test]
    async fn create_org_web_svc_rejects_owner_not_found() {
        let ctx = TestCtx::new("orgs_create_owner_missing")
            .await
            .expect("test ctx");
        let missing_owner = generate_id(IdPrefix::User);

        let csrf = create_csrf_token_svc("new_org", &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");

        let result = create_org_web_svc(
            &ctx.state,
            NewOrgFormData {
                token: csrf,
                name: "Platform Org".to_string(),
                owner_id: missing_owner,
                owner_email: "missing@example.com".to_string(),
            },
        )
        .await;

        assert!(result.is_err(), "missing owner should fail");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "Owner does not exists");
    }

    #[tokio::test]
    async fn create_org_web_svc_rejects_owner_superuser() {
        let ctx = TestCtx::new("orgs_create_owner_superuser")
            .await
            .expect("test ctx");
        let owner = ctx
            .seed_user_with_password(
                "Org Owner",
                "org.owner.superuser@example.com",
                "password123",
            )
            .await
            .expect("owner user");
        ctx.state
            .db
            .superusers
            .create(owner.id.clone())
            .await
            .expect("should create superuser");

        let csrf = create_csrf_token_svc("new_org", &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");

        let result = create_org_web_svc(
            &ctx.state,
            NewOrgFormData {
                token: csrf,
                name: "Platform Org".to_string(),
                owner_id: owner.id,
                owner_email: owner.email,
            },
        )
        .await;

        assert!(result.is_err(), "superuser owner should fail");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "Owner cannot be a superuser");
    }

    #[tokio::test]
    async fn update_org_web_svc_updates_org_and_get_returns_it() {
        let ctx = TestCtx::new("orgs_update_web").await.expect("test ctx");
        let fixture = ctx
            .seed_auth_fixture(
                "Owner User",
                "org.update.owner@example.com",
                "password123",
                "Original Org",
            )
            .await
            .expect("auth fixture");

        let csrf = create_csrf_token_svc(&fixture.org.id, &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");

        let updated = update_org_web_svc(
            &ctx.state,
            &fixture.org.id,
            UpdateOrgFormData {
                token: csrf,
                name: "Renamed Org".to_string(),
                active: None,
            },
        )
        .await
        .expect("org should be updated");

        assert_eq!(updated.name, "Renamed Org");
        assert_eq!(updated.status, "inactive");

        let fetched = get_org_svc(&ctx.state, &fixture.org.id)
            .await
            .expect("query should pass")
            .expect("org should exist");
        assert_eq!(fetched.name, "Renamed Org");
        assert_eq!(fetched.status, "inactive");
    }

    #[tokio::test]
    async fn update_org_web_svc_rejects_invalid_csrf_token() {
        let ctx = TestCtx::new("orgs_update_invalid_csrf")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_auth_fixture(
                "Owner User",
                "org.update.csrf.owner@example.com",
                "password123",
                "Original Org",
            )
            .await
            .expect("auth fixture");

        let result = update_org_web_svc(
            &ctx.state,
            &fixture.org.id,
            UpdateOrgFormData {
                token: "invalid.token".to_string(),
                name: "Renamed Org".to_string(),
                active: Some("1".to_string()),
            },
        )
        .await;

        assert!(result.is_err(), "invalid csrf should fail");
    }

    #[tokio::test]
    async fn update_org_owner_web_svc_rejects_new_owner_not_found() {
        let ctx = TestCtx::new("orgs_update_owner_missing")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_auth_fixture(
                "Owner User",
                "org.owner.missing.current@example.com",
                "password123",
                "Original Org",
            )
            .await
            .expect("auth fixture");
        let missing_owner = generate_id(IdPrefix::User);

        let csrf = create_csrf_token_svc(&fixture.org.id, &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");

        let result = update_org_owner_web_svc(
            &ctx.state,
            &fixture.org.id,
            UpdateOrgOwnerFormData {
                token: csrf,
                owner_id: missing_owner,
                owner_email: "missing@example.com".to_string(),
            },
        )
        .await;

        assert!(result.is_err(), "missing owner should fail");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "Owner does not exists");
    }

    #[tokio::test]
    async fn update_org_owner_web_svc_rejects_new_owner_not_member() {
        let ctx = TestCtx::new("orgs_update_owner_not_member")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_auth_fixture(
                "Owner User",
                "org.owner.not.member.current@example.com",
                "password123",
                "Original Org",
            )
            .await
            .expect("auth fixture");
        let external_user = ctx
            .seed_user_with_password(
                "External User",
                "org.owner.not.member@example.com",
                "password123",
            )
            .await
            .expect("external user");

        let csrf = create_csrf_token_svc(&fixture.org.id, &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");

        let result = update_org_owner_web_svc(
            &ctx.state,
            &fixture.org.id,
            UpdateOrgOwnerFormData {
                token: csrf,
                owner_id: external_user.id,
                owner_email: external_user.email,
            },
        )
        .await;

        assert!(result.is_err(), "non-member owner should fail");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "Owner must be a member of the org");
    }

    #[tokio::test]
    async fn update_org_owner_web_svc_rejects_new_owner_superuser() {
        let ctx = TestCtx::new("orgs_update_owner_superuser")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_auth_fixture(
                "Owner User",
                "org.owner.superuser.current@example.com",
                "password123",
                "Original Org",
            )
            .await
            .expect("auth fixture");
        let candidate_owner = ctx
            .seed_user_with_password(
                "Candidate Owner",
                "org.owner.superuser.new@example.com",
                "password123",
            )
            .await
            .expect("candidate owner");

        let member = ctx
            .state
            .db
            .org_members
            .create(
                fixture.org.id.clone(),
                crate::dto::NewOrgMemberDto {
                    user_id: candidate_owner.id.clone(),
                    roles: vec!["OrgAdmin".to_string()],
                    status: "active".to_string(),
                },
            )
            .await
            .expect("candidate owner should be member");

        assert_eq!(member.user_id, candidate_owner.id);

        ctx.state
            .db
            .superusers
            .create(candidate_owner.id.clone())
            .await
            .expect("should create superuser");

        let csrf = create_csrf_token_svc(&fixture.org.id, &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");

        let result = update_org_owner_web_svc(
            &ctx.state,
            &fixture.org.id,
            UpdateOrgOwnerFormData {
                token: csrf,
                owner_id: candidate_owner.id,
                owner_email: candidate_owner.email,
            },
        )
        .await;

        assert!(result.is_err(), "superuser owner should fail");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "Owner cannot be a superuser");
    }

    #[tokio::test]
    async fn delete_org_web_svc_deletes_org_and_get_returns_none() {
        let ctx = TestCtx::new("orgs_delete_web").await.expect("test ctx");
        let fixture = ctx
            .seed_auth_fixture(
                "Owner User",
                "org.delete.owner@example.com",
                "password123",
                "Disposable Org",
            )
            .await
            .expect("auth fixture");

        let owner_member = ctx
            .state
            .db
            .org_members
            .find_member(fixture.org.id.clone(), fixture.user.id.clone())
            .await
            .expect("membership query should pass")
            .expect("owner membership should exist");
        ctx.state
            .db
            .org_members
            .delete(owner_member.id)
            .await
            .expect("owner membership should be removed");

        let csrf = create_csrf_token_svc(&fixture.org.id, &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");
        delete_org_web_svc(&ctx.state, &fixture.org.id, &csrf)
            .await
            .expect("org should be deleted");

        let fetched = get_org_svc(&ctx.state, &fixture.org.id)
            .await
            .expect("query should pass");
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn delete_org_web_svc_rejects_when_org_has_members() {
        let ctx = TestCtx::new("orgs_delete_has_members")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_auth_fixture(
                "Owner User",
                "org.delete.members.owner@example.com",
                "password123",
                "Disposable Org",
            )
            .await
            .expect("auth fixture");

        let csrf = create_csrf_token_svc(&fixture.org.id, &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");
        let result = delete_org_web_svc(&ctx.state, &fixture.org.id, &csrf).await;

        assert!(result.is_err(), "org with members should fail to delete");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "Cannot delete org with existing members");
    }

    #[tokio::test]
    async fn delete_org_web_svc_rejects_when_org_has_linked_apps() {
        let ctx = TestCtx::new("orgs_delete_has_apps")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_auth_fixture(
                "Owner User",
                "org.delete.apps.owner@example.com",
                "password123",
                "Disposable Org",
            )
            .await
            .expect("auth fixture");

        let owner_member = ctx
            .state
            .db
            .org_members
            .find_member(fixture.org.id.clone(), fixture.user.id.clone())
            .await
            .expect("membership query should pass")
            .expect("owner membership should exist");
        ctx.state
            .db
            .org_members
            .delete(owner_member.id)
            .await
            .expect("owner membership should be removed");

        let app = ctx
            .seed_app("Linked App", "https://linked.example.com/callback")
            .await
            .expect("app should be created");
        create_org_app_svc(
            &ctx.state,
            &fixture.org.id,
            NewOrgAppDto {
                app_id: app.id.clone(),
            },
        )
        .await
        .expect("org app should be created");

        let csrf = create_csrf_token_svc(&fixture.org.id, &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");
        let result = delete_org_web_svc(&ctx.state, &fixture.org.id, &csrf).await;

        assert!(
            result.is_err(),
            "org with linked apps should fail to delete"
        );
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "Cannot delete org with existing apps");
    }
}
