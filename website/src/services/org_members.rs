use prost::Message;
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt, ensure};

use crate::ctx::Ctx;
use crate::error::{
    CsrfTokenSnafu, HttpClientSnafu, HttpResponseBytesSnafu, ProtobufDecodeSnafu, WhateverSnafu,
};
use crate::run::AppState;
use crate::services::token::verify_csrf_token;
use crate::{Error, Result};
use yaas::buffed::dto::{
    NewOrgBuf, OrgBuf, OrgMemberBuf, PaginatedOrgMembersBuf, PaginatedOrgsBuf, UpdateOrgBuf,
};
use yaas::dto::{ListOrgMembersParamsDto, ListOrgsParamsDto, OrgDto, OrgMemberDto};
use yaas::pagination::{Paginated, PaginatedMeta};

use super::handle_response_error;

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
        return Err(handle_response_error(response, "orgs", Error::OrgMemberNotFound).await);
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
        return Err(handle_response_error(response, "orgs", Error::OrgMemberNotFound).await);
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
