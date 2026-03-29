use serde::{Deserialize, Serialize};
use snafu::{ResultExt, ensure};
use yaas::role::to_roles;

use crate::ctx::Ctx;
use crate::error::{CsrfTokenSnafu, HttpClientSnafu, HttpResponseParseSnafu};
use crate::run::AppState;
use crate::services::token::verify_csrf_token;
use crate::{Error, Result};
use yaas::dto::{ListOrgMembersParamsDto, OrgMemberDto, OrgMemberSuggestionDto};
use yaas::pagination::Paginated;

use super::handle_response_error;

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
    ctx: &Ctx,
    org_id: &str,
    params: ListOrgMembersParamsDto,
) -> Result<Paginated<OrgMemberDto>> {
    let token = ctx.token().expect("Token is required");
    let url = format!("{}/orgs/{}/members", &state.config.api_url, org_id);

    let mut page = "1".to_string();
    let mut per_page = "10".to_string();

    if let Some(p) = params.page {
        page = p.to_string();
    }
    if let Some(pp) = params.per_page {
        per_page = pp.to_string();
    }
    let mut query: Vec<(&str, &str)> = vec![("page", &page), ("per_page", &per_page)];

    if let Some(keyword) = &params.keyword {
        query.push(("keyword", keyword));
    }

    let response = state
        .client
        .get(url)
        .bearer_auth(token)
        .query(&query)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to list org members. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "org_members", Error::OrgMemberNotFound).await);
    }

    response
        .json::<Paginated<OrgMemberDto>>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse org members response.".to_string(),
        })
}

pub async fn list_org_member_suggestions_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: &str,
    params: ListOrgMembersParamsDto,
) -> Result<Paginated<OrgMemberSuggestionDto>> {
    let token = ctx.token().expect("Token is required");
    let url = format!(
        "{}/orgs/{}/member-suggestions",
        &state.config.api_url, org_id
    );

    let mut page = "1".to_string();
    let mut per_page = "10".to_string();

    if let Some(p) = params.page {
        page = p.to_string();
    }
    if let Some(pp) = params.per_page {
        per_page = pp.to_string();
    }
    let mut query: Vec<(&str, &str)> = vec![("page", &page), ("per_page", &per_page)];

    if let Some(keyword) = &params.keyword {
        query.push(("keyword", keyword));
    }

    let response = state
        .client
        .get(url)
        .bearer_auth(token)
        .query(&query)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to list org member suggestions. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "org_members", Error::OrgMemberNotFound).await);
    }

    response
        .json::<Paginated<OrgMemberSuggestionDto>>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse org member suggestions response.".to_string(),
        })
}

pub async fn create_org_member_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: &str,
    form: NewOrgMemberFormData,
) -> Result<OrgMemberDto> {
    let token = ctx.token().expect("Token is required");
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == "new_org_member", CsrfTokenSnafu);

    let url = format!("{}/orgs/{}/members", &state.config.api_url, org_id);

    // Convert role to enum
    let Ok(roles) = to_roles(&[form.role]) else {
        return Err(Error::Validation {
            msg: "Role is invalid".to_string(),
        });
    };

    let body = serde_json::json!({
        "user_id": form.user_id,
        "roles": roles.iter().map(ToString::to_string).collect::<Vec<String>>(),
        "status": match form.active {
            Some(_) => "active",
            None => "inactive",
        },
    });

    let response = state
        .client
        .post(url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to create org member. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "org_members", Error::OrgMemberNotFound).await);
    }

    response
        .json::<OrgMemberDto>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse org member response.".to_string(),
        })
}

pub async fn get_org_member_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: &str,
    user_id: &str,
) -> Result<OrgMemberDto> {
    let token = ctx.token().expect("Token is required");
    let url = format!(
        "{}/orgs/{}/members/{}",
        &state.config.api_url, org_id, user_id
    );

    let response = state
        .client
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to get org member. Try again later.",
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "org_members", Error::OrgMemberNotFound).await);
    }

    response
        .json::<OrgMemberDto>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse org member response.".to_string(),
        })
}

pub async fn update_org_member_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: &str,
    user_id: &str,
    form: UpdateOrgMemberFormData,
) -> Result<OrgMemberDto> {
    let token = ctx.token().expect("Token is required");
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == user_id, CsrfTokenSnafu);

    let url = format!(
        "{}/orgs/{}/members/{}",
        &state.config.api_url, org_id, user_id
    );

    // Convert role to enum
    let Ok(roles) = to_roles(&[form.role]) else {
        return Err(Error::Validation {
            msg: "Role is invalid".to_string(),
        });
    };

    let body = serde_json::json!({
        "roles": roles.iter().map(ToString::to_string).collect::<Vec<String>>(),
        "status": match form.active {
            Some(_) => "active",
            None => "inactive",
        },
    });

    let response = state
        .client
        .patch(url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to update org member. Try again later.",
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "org_members", Error::OrgMemberNotFound).await);
    }

    response
        .json::<OrgMemberDto>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse org member response.".to_string(),
        })
}

pub async fn delete_org_member_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: &str,
    user_id: &str,
    csrf_token: &str,
) -> Result<()> {
    let token = ctx.token().expect("Token is required");

    let csrf_result = verify_csrf_token(csrf_token, &state.config.jwt_secret)?;
    ensure!(csrf_result == user_id, CsrfTokenSnafu);

    let url = format!(
        "{}/orgs/{}/members/{}",
        &state.config.api_url, org_id, user_id,
    );
    let response = state
        .client
        .delete(url)
        .bearer_auth(token)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to delete org member. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "org_members", Error::OrgMemberNotFound).await);
    }

    Ok(())
}
