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
