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
    NewOrgAppBuf, OrgAppBuf, PaginatedOrgAppSuggestionsBuf, PaginatedOrgAppsBuf,
};
use yaas::dto::{ListOrgAppsParamsDto, OrgAppDto, OrgAppSuggestionDto};
use yaas::pagination::{Paginated, PaginatedMeta};

use super::handle_response_error;

#[derive(Clone, Deserialize, Serialize)]
pub struct NewOrgAppFormData {
    pub token: String,
    pub app_id: i32,
    pub app_name: String,
}

pub async fn list_org_apps_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: i32,
    params: ListOrgAppsParamsDto,
) -> Result<Paginated<OrgAppDto>> {
    let token = ctx.token().expect("Token is required");
    let url = format!("{}/orgs/{}/apps", &state.config.api_url, org_id);

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
            msg: "Unable to list org apps. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "orgs", Error::OrgAppNotFound).await);
    }

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let listing = PaginatedOrgAppsBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;

    // Convert listing to dto
    let meta: PaginatedMeta = listing
        .meta
        .context(WhateverSnafu {
            msg: "Missing pagination metadata.".to_string(),
        })?
        .into();

    let org_apps: Vec<OrgAppDto> = listing.data.into_iter().map(|a| a.into()).collect();

    let dto: Paginated<OrgAppDto> = Paginated {
        meta,
        data: org_apps,
    };

    Ok(dto)
}

pub async fn list_org_app_suggestions_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: i32,
    params: ListOrgAppsParamsDto,
) -> Result<Paginated<OrgAppSuggestionDto>> {
    let token = ctx.token().expect("Token is required");
    let url = format!("{}/orgs/{}/app-suggestions", &state.config.api_url, org_id);

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
            msg: "Unable to list org app suggestions. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "org_apps", Error::OrgAppNotFound).await);
    }

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let listing =
        PaginatedOrgAppSuggestionsBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;

    // Convert listing to dto
    let meta: PaginatedMeta = listing
        .meta
        .context(WhateverSnafu {
            msg: "Missing pagination metadata.".to_string(),
        })?
        .into();

    let items: Vec<OrgAppSuggestionDto> = listing
        .data
        .into_iter()
        .map(|m| OrgAppSuggestionDto {
            id: m.id,
            name: m.name,
        })
        .collect();

    let dto: Paginated<OrgAppSuggestionDto> = Paginated { meta, data: items };

    Ok(dto)
}

pub async fn create_org_app_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: i32,
    form: NewOrgAppFormData,
) -> Result<OrgAppDto> {
    let token = ctx.token().expect("Token is required");
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == "new_org_app", CsrfTokenSnafu);

    let url = format!("{}/orgs/{}/apps", &state.config.api_url, org_id);

    let body = NewOrgAppBuf {
        app_id: form.app_id,
    };

    let response = state
        .client
        .post(url)
        .bearer_auth(token)
        .body(prost::Message::encode_to_vec(&body))
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to create org app. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "org_apps", Error::OrgAppNotFound).await);
    }

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let app = OrgAppBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;

    Ok(app.into())
}

pub async fn get_org_app_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: i32,
    app_id: i32,
) -> Result<OrgAppDto> {
    let token = ctx.token().expect("Token is required");
    let url = format!("{}/orgs/{}/apps/{}", &state.config.api_url, org_id, app_id);

    let response = state
        .client
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to get org app. Try again later.",
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "org_apps", Error::OrgAppNotFound).await);
    }

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let org_app = OrgAppBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;

    Ok(org_app.into())
}

pub async fn delete_org_app_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: i32,
    app_id: i32,
    csrf_token: &str,
) -> Result<()> {
    let token = ctx.token().expect("Token is required");

    let csrf_result = verify_csrf_token(csrf_token, &state.config.jwt_secret)?;
    ensure!(csrf_result == app_id.to_string(), CsrfTokenSnafu);

    let url = format!("{}/orgs/{}/apps/{}", &state.config.api_url, org_id, app_id,);
    let response = state
        .client
        .delete(url)
        .bearer_auth(token)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to delete org app. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "org_apps", Error::OrgAppNotFound).await);
    }

    Ok(())
}
