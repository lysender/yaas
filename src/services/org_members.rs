use serde::{Deserialize, Serialize};
use snafu::ensure;

use crate::dto::ListingParamsDto;
use crate::dto::OrgMembershipDto;
use crate::dto::Paginated;
use crate::dto::to_roles;
use crate::dto::{
    ListOrgMembersParamsDto, NewOrgMemberDto, OrgMemberDto, OrgMemberSuggestionDto,
    UpdateOrgMemberDto,
};
use crate::error::CsrfTokenSnafu;
use crate::error::ValidationSnafu;
use crate::run::AppState;
use crate::services::token::verify_csrf_token;
use crate::{Error, Result};

#[derive(Clone, Deserialize, Serialize)]
pub struct NewOrgMemberFormData {
    pub token: String,
    pub user_id: String,
    pub user_email: String,
    pub role: String,
    pub active: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UpdateOrgMemberFormData {
    pub token: String,
    pub role: String,
    pub active: Option<String>,
}

pub async fn list_org_members_svc(
    state: &AppState,
    org_id: &str,
    params: ListOrgMembersParamsDto,
) -> Result<Paginated<OrgMemberDto>> {
    state.db.org_members.list(org_id.to_string(), params).await
}

pub async fn list_org_member_suggestions_svc(
    state: &AppState,
    org_id: &str,
    params: ListOrgMembersParamsDto,
) -> Result<Paginated<OrgMemberSuggestionDto>> {
    state
        .db
        .org_members
        .list_member_suggestions(org_id.to_string(), params)
        .await
}

pub async fn list_org_memberships_svc(
    state: &AppState,
    user_id: &str,
    params: ListingParamsDto,
) -> Result<Paginated<OrgMembershipDto>> {
    state
        .db
        .org_members
        .list_memberships(user_id.to_string(), params)
        .await
}

pub async fn create_org_member_svc(
    state: &AppState,
    org_id: &str,
    data: NewOrgMemberDto,
) -> Result<OrgMemberDto> {
    // Ensure that the user exists
    let user_id = data.user_id.clone();
    let existing_user = state.db.users.get(user_id.clone()).await?;

    ensure!(
        existing_user.is_some(),
        ValidationSnafu {
            msg: "User does not exist".to_string(),
        }
    );

    // Ensure user is not already a member of the org
    let existing_member = state
        .db
        .org_members
        .find_member(org_id.to_string(), user_id.clone())
        .await?;

    ensure!(
        existing_member.is_none(),
        ValidationSnafu {
            msg: "User is already a member of the organization".to_string(),
        }
    );

    // Do not allow adding superusers as org members
    let superuser = state.db.superusers.get(user_id).await?;

    ensure!(
        superuser.is_none(),
        ValidationSnafu {
            msg: "Cannot add superuser as organization member".to_string(),
        }
    );

    state.db.org_members.create(org_id.to_string(), data).await
}

pub async fn create_org_member_web_svc(
    state: &AppState,
    org_id: &str,
    form: NewOrgMemberFormData,
) -> Result<OrgMemberDto> {
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == "new_org_member", CsrfTokenSnafu);

    // Convert role to enum
    let Ok(roles) = to_roles(&[form.role]) else {
        return Err(Error::Validation {
            msg: "Role is invalid".to_string(),
        });
    };

    create_org_member_svc(
        state,
        org_id,
        NewOrgMemberDto {
            user_id: form.user_id,
            roles: roles.into_iter().map(|r| r.to_string()).collect(),
            status: match form.active {
                Some(_) => "active".to_string(),
                None => "inactive".to_string(),
            },
        },
    )
    .await
}

pub async fn get_org_member_svc(
    state: &AppState,
    org_id: &str,
    user_id: &str,
) -> Result<Option<OrgMemberDto>> {
    state
        .db
        .org_members
        .find_member(org_id.to_string(), user_id.to_string())
        .await
}

pub async fn update_org_member_svc(
    state: &AppState,
    id: &str,
    data: UpdateOrgMemberDto,
) -> Result<bool> {
    state.db.org_members.update(id.to_string(), data).await
}

pub async fn update_org_member_web_svc(
    state: &AppState,
    org_id: &str,
    user_id: &str,
    form: UpdateOrgMemberFormData,
) -> Result<OrgMemberDto> {
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == user_id, CsrfTokenSnafu);

    // Convert role to enum
    let Ok(roles) = to_roles(&[form.role]) else {
        return Err(Error::Validation {
            msg: "Role is invalid".to_string(),
        });
    };

    // Find member entry
    let Some(member) = get_org_member_svc(state, org_id, user_id).await? else {
        return Err(Error::OrgMemberNotFound);
    };

    update_org_member_svc(
        state,
        &member.id,
        UpdateOrgMemberDto {
            roles: Some(roles.into_iter().map(|r| r.to_string()).collect()),
            status: match form.active {
                Some(_) => Some("active".to_string()),
                None => Some("inactive".to_string()),
            },
        },
    )
    .await?;

    // Fetch the updated member to return
    let Some(updated_member) = get_org_member_svc(state, org_id, user_id).await? else {
        return Err(Error::OrgMemberNotFound);
    };

    Ok(updated_member)
}

pub async fn delete_org_member_svc(state: &AppState, id: &str) -> Result<()> {
    state.db.org_members.delete(id.to_string()).await
}

pub async fn delete_org_member_web_svc(
    state: &AppState,
    org_id: &str,
    user_id: &str,
    csrf_token: &str,
) -> Result<()> {
    let csrf_result = verify_csrf_token(csrf_token, &state.config.jwt_secret)?;
    ensure!(csrf_result == user_id, CsrfTokenSnafu);

    // Find member entry
    let Some(member) = get_org_member_svc(state, org_id, user_id).await? else {
        return Err(Error::OrgMemberNotFound);
    };

    delete_org_member_svc(state, &member.id).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::services::token::create_csrf_token_svc;
    use crate::test::TestCtx;
    use crate::utils::{IdPrefix, generate_id};

    use super::{
        NewOrgMemberFormData, UpdateOrgMemberFormData, create_org_member_web_svc,
        delete_org_member_web_svc, get_org_member_svc, update_org_member_web_svc,
    };

    #[tokio::test]
    async fn create_org_member_web_svc_creates_member_and_get_returns_it() {
        let ctx = TestCtx::new("org_members_create_web")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_auth_fixture(
                "Owner User",
                "org.members.owner@example.com",
                "password123",
                "Org Members Org",
            )
            .await
            .expect("auth fixture");
        let member_user = ctx
            .seed_user_with_password("Member User", "org.member@example.com", "password123")
            .await
            .expect("member user");

        let csrf = create_csrf_token_svc("new_org_member", &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");

        let created = create_org_member_web_svc(
            &ctx.state,
            &fixture.org.id,
            NewOrgMemberFormData {
                token: csrf,
                user_id: member_user.id.clone(),
                user_email: member_user.email.clone(),
                role: "OrgEditor".to_string(),
                active: Some("1".to_string()),
            },
        )
        .await
        .expect("member should be created");

        let fetched = get_org_member_svc(&ctx.state, &fixture.org.id, &member_user.id)
            .await
            .expect("query should pass")
            .expect("member should exist");

        assert_eq!(created.id, fetched.id);
        assert_eq!(fetched.user_id, member_user.id);
        assert_eq!(fetched.status, "active");
    }

    #[tokio::test]
    async fn create_org_member_web_svc_rejects_invalid_csrf_token() {
        let ctx = TestCtx::new("org_members_create_invalid_csrf")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_auth_fixture(
                "Owner User",
                "org.members.owner.csrf@example.com",
                "password123",
                "Org Members Org",
            )
            .await
            .expect("auth fixture");
        let member_user = ctx
            .seed_user_with_password("Member User", "org.member.csrf@example.com", "password123")
            .await
            .expect("member user");

        let result = create_org_member_web_svc(
            &ctx.state,
            &fixture.org.id,
            NewOrgMemberFormData {
                token: "invalid.token".to_string(),
                user_id: member_user.id,
                user_email: member_user.email,
                role: "OrgEditor".to_string(),
                active: Some("1".to_string()),
            },
        )
        .await;

        assert!(result.is_err(), "invalid csrf should fail");
    }

    #[tokio::test]
    async fn create_org_member_web_svc_rejects_invalid_role() {
        let ctx = TestCtx::new("org_members_create_invalid_role")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_auth_fixture(
                "Owner User",
                "org.members.owner.role@example.com",
                "password123",
                "Org Members Org",
            )
            .await
            .expect("auth fixture");
        let member_user = ctx
            .seed_user_with_password("Member User", "org.member.role@example.com", "password123")
            .await
            .expect("member user");

        let csrf = create_csrf_token_svc("new_org_member", &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");

        let result = create_org_member_web_svc(
            &ctx.state,
            &fixture.org.id,
            NewOrgMemberFormData {
                token: csrf,
                user_id: member_user.id,
                user_email: member_user.email,
                role: "InvalidRole".to_string(),
                active: Some("1".to_string()),
            },
        )
        .await;

        assert!(result.is_err(), "invalid role should fail");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "Role is invalid");
    }

    #[tokio::test]
    async fn update_org_member_web_svc_updates_member_and_get_returns_it() {
        let ctx = TestCtx::new("org_members_update_web")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_auth_fixture(
                "Owner User",
                "org.members.owner.update@example.com",
                "password123",
                "Org Members Org",
            )
            .await
            .expect("auth fixture");
        let member_user = ctx
            .seed_user_with_password(
                "Member User",
                "org.member.update@example.com",
                "password123",
            )
            .await
            .expect("member user");

        let create_csrf = create_csrf_token_svc("new_org_member", &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");
        create_org_member_web_svc(
            &ctx.state,
            &fixture.org.id,
            NewOrgMemberFormData {
                token: create_csrf,
                user_id: member_user.id.clone(),
                user_email: member_user.email,
                role: "OrgViewer".to_string(),
                active: Some("1".to_string()),
            },
        )
        .await
        .expect("member should be created");

        let update_csrf = create_csrf_token_svc(&member_user.id, &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");
        update_org_member_web_svc(
            &ctx.state,
            &fixture.org.id,
            &member_user.id,
            UpdateOrgMemberFormData {
                token: update_csrf,
                role: "OrgAdmin".to_string(),
                active: None,
            },
        )
        .await
        .expect("member should be updated");

        let fetched = get_org_member_svc(&ctx.state, &fixture.org.id, &member_user.id)
            .await
            .expect("query should pass")
            .expect("member should exist");

        assert_eq!(fetched.status, "inactive");
        assert_eq!(
            fetched.roles.first().map(|r| r.to_string()),
            Some("OrgAdmin".to_string())
        );
    }

    #[tokio::test]
    async fn update_org_member_web_svc_rejects_invalid_csrf_token() {
        let ctx = TestCtx::new("org_members_update_invalid_csrf")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_auth_fixture(
                "Owner User",
                "org.members.owner.update.csrf@example.com",
                "password123",
                "Org Members Org",
            )
            .await
            .expect("auth fixture");
        let member_user = ctx
            .seed_user_with_password(
                "Member User",
                "org.member.update.csrf@example.com",
                "password123",
            )
            .await
            .expect("member user");

        let create_csrf = create_csrf_token_svc("new_org_member", &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");
        create_org_member_web_svc(
            &ctx.state,
            &fixture.org.id,
            NewOrgMemberFormData {
                token: create_csrf,
                user_id: member_user.id.clone(),
                user_email: member_user.email,
                role: "OrgViewer".to_string(),
                active: Some("1".to_string()),
            },
        )
        .await
        .expect("member should be created");

        let result = update_org_member_web_svc(
            &ctx.state,
            &fixture.org.id,
            &member_user.id,
            UpdateOrgMemberFormData {
                token: "invalid.token".to_string(),
                role: "OrgAdmin".to_string(),
                active: None,
            },
        )
        .await;

        assert!(result.is_err(), "invalid csrf should fail");
    }

    #[tokio::test]
    async fn update_org_member_web_svc_rejects_invalid_role() {
        let ctx = TestCtx::new("org_members_update_invalid_role")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_auth_fixture(
                "Owner User",
                "org.members.owner.update.role@example.com",
                "password123",
                "Org Members Org",
            )
            .await
            .expect("auth fixture");
        let member_user = ctx
            .seed_user_with_password(
                "Member User",
                "org.member.update.role@example.com",
                "password123",
            )
            .await
            .expect("member user");

        let create_csrf = create_csrf_token_svc("new_org_member", &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");
        create_org_member_web_svc(
            &ctx.state,
            &fixture.org.id,
            NewOrgMemberFormData {
                token: create_csrf,
                user_id: member_user.id.clone(),
                user_email: member_user.email,
                role: "OrgViewer".to_string(),
                active: Some("1".to_string()),
            },
        )
        .await
        .expect("member should be created");

        let update_csrf = create_csrf_token_svc(&member_user.id, &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");

        let result = update_org_member_web_svc(
            &ctx.state,
            &fixture.org.id,
            &member_user.id,
            UpdateOrgMemberFormData {
                token: update_csrf,
                role: "InvalidRole".to_string(),
                active: None,
            },
        )
        .await;

        assert!(result.is_err(), "invalid role should fail");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "Role is invalid");
    }

    #[tokio::test]
    async fn update_org_member_web_svc_rejects_member_not_found() {
        let ctx = TestCtx::new("org_members_update_missing_member")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_auth_fixture(
                "Owner User",
                "org.members.owner.update.missing@example.com",
                "password123",
                "Org Members Org",
            )
            .await
            .expect("auth fixture");
        let missing_user_id = generate_id(IdPrefix::User);

        let csrf = create_csrf_token_svc(&missing_user_id, &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");

        let result = update_org_member_web_svc(
            &ctx.state,
            &fixture.org.id,
            &missing_user_id,
            UpdateOrgMemberFormData {
                token: csrf,
                role: "OrgAdmin".to_string(),
                active: None,
            },
        )
        .await;

        assert!(result.is_err(), "missing member should fail");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "Org member not found");
    }

    #[tokio::test]
    async fn delete_org_member_web_svc_deletes_member_and_get_returns_none() {
        let ctx = TestCtx::new("org_members_delete_web")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_auth_fixture(
                "Owner User",
                "org.members.owner.delete@example.com",
                "password123",
                "Org Members Org",
            )
            .await
            .expect("auth fixture");
        let member_user = ctx
            .seed_user_with_password(
                "Member User",
                "org.member.delete@example.com",
                "password123",
            )
            .await
            .expect("member user");

        let create_csrf = create_csrf_token_svc("new_org_member", &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");
        create_org_member_web_svc(
            &ctx.state,
            &fixture.org.id,
            NewOrgMemberFormData {
                token: create_csrf,
                user_id: member_user.id.clone(),
                user_email: member_user.email,
                role: "OrgViewer".to_string(),
                active: Some("1".to_string()),
            },
        )
        .await
        .expect("member should be created");

        let delete_csrf = create_csrf_token_svc(&member_user.id, &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");
        delete_org_member_web_svc(&ctx.state, &fixture.org.id, &member_user.id, &delete_csrf)
            .await
            .expect("member should be deleted");

        let fetched = get_org_member_svc(&ctx.state, &fixture.org.id, &member_user.id)
            .await
            .expect("query should pass");
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn delete_org_member_web_svc_rejects_invalid_csrf_token() {
        let ctx = TestCtx::new("org_members_delete_invalid_csrf")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_auth_fixture(
                "Owner User",
                "org.members.owner.delete.csrf@example.com",
                "password123",
                "Org Members Org",
            )
            .await
            .expect("auth fixture");
        let member_user = ctx
            .seed_user_with_password(
                "Member User",
                "org.member.delete.csrf@example.com",
                "password123",
            )
            .await
            .expect("member user");

        let create_csrf = create_csrf_token_svc("new_org_member", &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");
        create_org_member_web_svc(
            &ctx.state,
            &fixture.org.id,
            NewOrgMemberFormData {
                token: create_csrf,
                user_id: member_user.id.clone(),
                user_email: member_user.email,
                role: "OrgViewer".to_string(),
                active: Some("1".to_string()),
            },
        )
        .await
        .expect("member should be created");

        let result = delete_org_member_web_svc(
            &ctx.state,
            &fixture.org.id,
            &member_user.id,
            "invalid.token",
        )
        .await;

        assert!(result.is_err(), "invalid csrf should fail");
    }

    #[tokio::test]
    async fn delete_org_member_web_svc_rejects_member_not_found() {
        let ctx = TestCtx::new("org_members_delete_missing_member")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_auth_fixture(
                "Owner User",
                "org.members.owner.delete.missing@example.com",
                "password123",
                "Org Members Org",
            )
            .await
            .expect("auth fixture");
        let missing_user_id = generate_id(IdPrefix::User);

        let csrf = create_csrf_token_svc(&missing_user_id, &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");

        let result =
            delete_org_member_web_svc(&ctx.state, &fixture.org.id, &missing_user_id, &csrf).await;

        assert!(result.is_err(), "missing member should fail");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "Org member not found");
    }
}
