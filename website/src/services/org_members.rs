use prost::Message;
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt, ensure};
use yaas::role::{Role, to_buffed_roles, to_roles};

use crate::ctx::Ctx;
use crate::error::{
    CsrfTokenSnafu, HttpClientSnafu, HttpResponseBytesSnafu, ProtobufDecodeSnafu, ValidationSnafu,
    WhateverSnafu,
};
use crate::run::AppState;
use crate::services::NewAppFormData;
use crate::services::token::verify_csrf_token;
use crate::{Error, Result};
use yaas::buffed::dto::{
    NewOrgBuf, NewOrgMemberBuf, OrgBuf, OrgMemberBuf, OrgMemberSuggestionBuf,
    PaginatedOrgMemberSuggestionsBuf, PaginatedOrgMembersBuf, PaginatedOrgsBuf, UpdateOrgBuf,
    UpdateOrgMemberBuf,
};
use yaas::dto::{
    ListOrgMembersParamsDto, ListOrgsParamsDto, OrgDto, OrgMemberDto, OrgMemberSuggestionDto,
    UserDto,
};
use yaas::pagination::{Paginated, PaginatedMeta};

use super::handle_response_error;

#[derive(Clone, Deserialize, Serialize)]
pub struct NewOrgMemberFormData {
    pub token: String,
    pub user_id: i32,
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
    org_id: i32,
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

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let listing =
        PaginatedOrgMembersBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;

    // Convert listing to dto
    let meta: PaginatedMeta = listing
        .meta
        .context(WhateverSnafu {
            msg: "Missing pagination metadata.".to_string(),
        })?
        .into();

    let try_members: std::result::Result<Vec<OrgMemberDto>, String> =
        listing.data.into_iter().map(|m| m.try_into()).collect();

    match try_members {
        Err(e) => Err(Error::Service {
            msg: format!("Unable to parse org members: {}", e),
        }),
        Ok(org_members) => {
            let dto: Paginated<OrgMemberDto> = Paginated {
                meta,
                data: org_members,
            };

            Ok(dto)
        }
    }
}

pub async fn list_org_member_suggestions_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: i32,
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

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let listing = PaginatedOrgMemberSuggestionsBuf::decode(&body_bytes[..])
        .context(ProtobufDecodeSnafu {})?;

    // Convert listing to dto
    let meta: PaginatedMeta = listing
        .meta
        .context(WhateverSnafu {
            msg: "Missing pagination metadata.".to_string(),
        })?
        .into();

    let items: Vec<OrgMemberSuggestionDto> = listing
        .data
        .into_iter()
        .map(|m| OrgMemberSuggestionDto {
            id: m.id,
            email: m.email,
            name: m.name,
        })
        .collect();

    let dto: Paginated<OrgMemberSuggestionDto> = Paginated { meta, data: items };

    Ok(dto)
}

pub async fn create_org_member_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: i32,
    form: NewOrgMemberFormData,
) -> Result<OrgMemberDto> {
    let token = ctx.token().expect("Token is required");
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == "new_org_member", CsrfTokenSnafu);

    let url = format!("{}/orgs/{}/members", &state.config.api_url, org_id);

    // Convert role to enum
    let Ok(roles) = to_roles(&vec![form.role.clone()]) else {
        return Err(Error::Validation {
            msg: "Role is invalid".to_string(),
        });
    };

    let body = NewOrgMemberBuf {
        user_id: form.user_id,
        roles: to_buffed_roles(&roles),
        status: match form.active {
            Some(_) => "active".to_string(),
            None => "inactive".to_string(),
        },
    };

    let response = state
        .client
        .post(url)
        .bearer_auth(token)
        .body(prost::Message::encode_to_vec(&body))
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to create org member. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "org_members", Error::OrgMemberNotFound).await);
    }

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let member = OrgMemberBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;

    match member.try_into() {
        Ok(dto) => Ok(dto),
        Err(_) => Err(Error::Whatever {
            msg: "Unable to decode org member".to_string(),
        }),
    }
}

pub async fn get_org_member_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: i32,
    user_id: i32,
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

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let org_member = OrgMemberBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;
    match org_member.try_into() {
        Err(e) => Err(Error::Service {
            msg: format!("Unable to parse org member: {}", e),
        }),
        Ok(dto) => Ok(dto),
    }
}

pub async fn update_org_member_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: i32,
    user_id: i32,
    form: UpdateOrgMemberFormData,
) -> Result<OrgMemberDto> {
    let token = ctx.token().expect("Token is required");
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(&csrf_result == &user_id.to_string(), CsrfTokenSnafu);

    let url = format!(
        "{}/orgs/{}/members/{}",
        &state.config.api_url, org_id, user_id
    );

    // Convert role to enum
    let Ok(roles) = to_roles(&vec![form.role.clone()]) else {
        return Err(Error::Validation {
            msg: "Role is invalid".to_string(),
        });
    };

    let body = UpdateOrgMemberBuf {
        roles: to_buffed_roles(&roles),
        status: match form.active {
            Some(_) => Some("active".to_string()),
            None => Some("inactive".to_string()),
        },
    };

    let response = state
        .client
        .patch(url)
        .bearer_auth(token)
        .body(prost::Message::encode_to_vec(&body))
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to update org member. Try again later.",
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "org_members", Error::OrgMemberNotFound).await);
    }

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let member = OrgMemberBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;

    match member.try_into() {
        Ok(dto) => Ok(dto),
        Err(_) => Err(Error::Whatever {
            msg: "Unable to decode org member".to_string(),
        }),
    }
}

pub async fn delete_org_member_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: i32,
    user_id: i32,
    csrf_token: &str,
) -> Result<()> {
    let token = ctx.token().expect("Token is required");

    let csrf_result = verify_csrf_token(&csrf_token, &state.config.jwt_secret)?;
    ensure!(&csrf_result == &user_id.to_string(), CsrfTokenSnafu);

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
