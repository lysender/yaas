use prost::Message;
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt, ensure};
use yaas::buffed::dto::{AppBuf, NewAppBuf, PaginatedAppsBuf, UpdateAppBuf};
use yaas::pagination::{Paginated, PaginatedMeta};

use crate::ctx::Ctx;
use crate::error::{
    CsrfTokenSnafu, HttpClientSnafu, HttpResponseBytesSnafu, ProtobufDecodeSnafu, WhateverSnafu,
};
use crate::run::AppState;
use crate::services::token::verify_csrf_token;
use crate::{Error, Result};
use yaas::dto::{AppDto, ListAppsParamsDto};

use super::handle_response_error;

#[derive(Clone, Deserialize, Serialize)]
pub struct NewAppFormData {
    pub name: String,
    pub redirect_uri: String,
    pub token: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UpdateAppFormData {
    pub token: String,
    pub name: String,
    pub redirect_uri: String,
}

pub async fn list_apps_svc(
    state: &AppState,
    ctx: &Ctx,
    params: ListAppsParamsDto,
) -> Result<Paginated<AppDto>> {
    let token = ctx.token().expect("Token is required");
    let url = format!("{}/apps", &state.config.api_url);

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
            msg: "Unable to list apps. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "apps", Error::AppNotFound).await);
    }

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let listing = PaginatedAppsBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;

    // Convert listing to dto
    let meta: PaginatedMeta = listing
        .meta
        .context(WhateverSnafu {
            msg: "Missing pagination metadata.".to_string(),
        })?
        .into();

    let apps: Vec<AppDto> = listing.data.into_iter().map(|u| u.into()).collect();
    let dto: Paginated<AppDto> = Paginated { meta, data: apps };

    Ok(dto)
}

pub async fn create_app_svc(state: &AppState, ctx: &Ctx, form: NewAppFormData) -> Result<AppDto> {
    let token = ctx.token().expect("Token is required");
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == "new_app", CsrfTokenSnafu);

    let url = format!("{}/apps", &state.config.api_url);

    let body = NewAppBuf {
        name: form.name,
        redirect_uri: form.redirect_uri,
    };

    let response = state
        .client
        .post(url)
        .bearer_auth(token)
        .body(prost::Message::encode_to_vec(&body))
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to create app. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "apps", Error::AppNotFound).await);
    }

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let app = AppBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;
    let dto: AppDto = app.into();

    Ok(dto)
}

pub async fn get_app_svc(state: &AppState, ctx: &Ctx, app_id: i32) -> Result<AppDto> {
    let token = ctx.token().expect("Token is required");
    let url = format!("{}/apps/{}", &state.config.api_url, app_id);

    let response = state
        .client
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to get app. Try again later.",
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "apps", Error::AppNotFound).await);
    }

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let app = AppBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;
    let dto: AppDto = app.into();

    Ok(dto)
}

pub async fn update_app_svc(
    state: &AppState,
    ctx: &Ctx,
    app_id: i32,
    form: UpdateAppFormData,
) -> Result<AppDto> {
    let token = ctx.token().expect("Token is required");
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == app_id.to_string(), CsrfTokenSnafu);

    let url = format!("{}/apps/{}", &state.config.api_url, app_id);
    let body = UpdateAppBuf {
        name: Some(form.name),
        redirect_uri: Some(form.redirect_uri),
    };
    let response = state
        .client
        .patch(url)
        .bearer_auth(token)
        .body(prost::Message::encode_to_vec(&body))
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to update app. Try again later.",
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "apps", Error::AppNotFound).await);
    }

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let app = AppBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;
    let dto: AppDto = app.into();

    Ok(dto)
}

pub async fn regenerate_app_secret_svc(
    state: &AppState,
    ctx: &Ctx,
    app_id: i32,
    csrf_token: &str,
) -> Result<()> {
    let token = ctx.token().expect("Token is required");

    let csrf_result = verify_csrf_token(csrf_token, &state.config.jwt_secret)?;
    ensure!(csrf_result == app_id.to_string(), CsrfTokenSnafu);

    let url = format!(
        "{}/apps/{}/regenerate-secret",
        &state.config.api_url, app_id,
    );
    let response = state
        .client
        .post(url)
        .bearer_auth(token)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to regenerate app secret. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "apps", Error::AppNotFound).await);
    }

    Ok(())
}

pub async fn delete_app_svc(
    state: &AppState,
    ctx: &Ctx,
    app_id: i32,
    csrf_token: &str,
) -> Result<()> {
    let token = ctx.token().expect("Token is required");

    let csrf_result = verify_csrf_token(csrf_token, &state.config.jwt_secret)?;
    ensure!(csrf_result == app_id.to_string(), CsrfTokenSnafu);

    let url = format!("{}/apps/{}", &state.config.api_url, app_id,);
    let response = state
        .client
        .delete(url)
        .bearer_auth(token)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to delete app. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "apps", Error::AppNotFound).await);
    }

    Ok(())
}
