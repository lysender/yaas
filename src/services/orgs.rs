use serde::{Deserialize, Serialize};
use snafu::ensure;

use crate::ctx::Ctx;
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

pub async fn delete_org_web_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: &str,
    csrf_token: &str,
) -> Result<()> {
    let token = ctx.token().expect("Token is required");

    let csrf_result = verify_csrf_token(csrf_token, &state.config.jwt_secret)?;
    ensure!(csrf_result == org_id, CsrfTokenSnafu);

    delete_org_svc(state, org_id).await?;

    Ok(())
}
