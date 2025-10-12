use prost::Message;
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt, ensure};
use yaas::buffed::dto::{NewOrgBuf, OrgBuf, PaginatedOrgsBuf, UpdateOrgBuf};
use yaas::pagination::{Paginated, PaginatedMeta};

use crate::ctx::Ctx;
use crate::error::{
    CsrfTokenSnafu, HttpClientSnafu, HttpResponseBytesSnafu, ProtobufDecodeSnafu, WhateverSnafu,
};
use crate::run::AppState;
use crate::services::token::verify_csrf_token;
use crate::{Error, Result};
use yaas::dto::{ListOrgsParamsDto, OrgDto};

use super::handle_response_error;

#[derive(Clone, Deserialize, Serialize)]
pub struct NewOrgFormData {
    pub token: String,
    pub name: String,
    pub owner_id: i32,
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
    pub owner_id: i32,
    pub owner_email: String,
}

pub async fn list_orgs_svc(
    state: &AppState,
    ctx: &Ctx,
    params: ListOrgsParamsDto,
) -> Result<Paginated<OrgDto>> {
    let token = ctx.token().expect("Token is required");
    let url = format!("{}/orgs", &state.config.api_url);

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
            msg: "Unable to list orgs. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "orgs", Error::OrgNotFound).await);
    }

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let listing = PaginatedOrgsBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;

    // Convert listing to dto
    let meta: PaginatedMeta = listing
        .meta
        .context(WhateverSnafu {
            msg: "Missing pagination metadata.".to_string(),
        })?
        .into();

    let orgs: Vec<OrgDto> = listing.data.into_iter().map(|u| u.into()).collect();
    let dto: Paginated<OrgDto> = Paginated { meta, data: orgs };

    Ok(dto)
}

pub async fn create_org_svc(state: &AppState, ctx: &Ctx, form: NewOrgFormData) -> Result<OrgDto> {
    let token = ctx.token().expect("Token is required");
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == "new_org", CsrfTokenSnafu);

    let url = format!("{}/orgs", &state.config.api_url);

    let body = NewOrgBuf {
        name: form.name,
        owner_id: form.owner_id,
    };

    let response = state
        .client
        .post(url)
        .bearer_auth(token)
        .body(prost::Message::encode_to_vec(&body))
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to create org. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "orgs", Error::OrgNotFound).await);
    }

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let org = OrgBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;
    let dto: OrgDto = org.into();

    Ok(dto)
}

pub async fn get_org_svc(state: &AppState, ctx: &Ctx, org_id: &str) -> Result<OrgDto> {
    let token = ctx.token().expect("Token is required");
    let url = format!("{}/orgs/{}", &state.config.api_url, org_id);

    let response = state
        .client
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to get org. Try again later.",
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "orgs", Error::OrgNotFound).await);
    }

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let org = OrgBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;
    let dto: OrgDto = org.into();

    Ok(dto)
}

pub async fn update_org_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: i32,
    form: UpdateOrgFormData,
) -> Result<OrgDto> {
    let token = ctx.token().expect("Token is required");
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(&csrf_result == &org_id.to_string(), CsrfTokenSnafu);

    let url = format!("{}/orgs/{}", &state.config.api_url, org_id);
    let body = UpdateOrgBuf {
        name: Some(form.name),
        owner_id: None,
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
            msg: "Unable to update org. Try again later.",
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "orgs", Error::OrgNotFound).await);
    }

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let org = OrgBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;
    let dto: OrgDto = org.into();

    Ok(dto)
}

pub async fn update_org_owner_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: i32,
    form: UpdateOrgOwnerFormData,
) -> Result<OrgDto> {
    let token = ctx.token().expect("Token is required");
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(&csrf_result == &org_id.to_string(), CsrfTokenSnafu);

    let url = format!("{}/orgs/{}", &state.config.api_url, org_id);
    let body = UpdateOrgBuf {
        name: None,
        owner_id: Some(form.owner_id),
        status: None,
    };
    let response = state
        .client
        .patch(url)
        .bearer_auth(token)
        .body(prost::Message::encode_to_vec(&body))
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to update org. Try again later.",
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "orgs", Error::OrgNotFound).await);
    }

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let org = OrgBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;
    let dto: OrgDto = org.into();

    Ok(dto)
}

pub async fn delete_org_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: i32,
    csrf_token: &str,
) -> Result<()> {
    let token = ctx.token().expect("Token is required");

    let csrf_result = verify_csrf_token(&csrf_token, &state.config.jwt_secret)?;
    ensure!(&csrf_result == &org_id.to_string(), CsrfTokenSnafu);

    let url = format!("{}/orgs/{}", &state.config.api_url, org_id,);
    let response = state
        .client
        .delete(url)
        .bearer_auth(token)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to delete org. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "orgs", Error::OrgNotFound).await);
    }

    Ok(())
}
