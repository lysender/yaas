use serde::{Deserialize, Serialize};
use snafu::{ResultExt, ensure};

use crate::ctx::Ctx;
use crate::dto::Paginated;
use crate::dto::{ListOrgAppsParamsDto, NewOrgAppDto, OrgAppDto, OrgAppSuggestionDto};
use crate::error::{CsrfTokenSnafu, HttpClientSnafu, HttpResponseParseSnafu};
use crate::run::AppState;
use crate::services::token::verify_csrf_token;
use crate::{Error, Result};

use super::handle_response_error;

#[derive(Clone, Deserialize, Serialize)]
pub struct NewOrgAppFormData {
    pub token: String,
    pub app_id: String,
    pub app_name: String,
}

pub async fn list_org_apps_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: &str,
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

    response
        .json::<Paginated<OrgAppDto>>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse org apps response.".to_string(),
        })
}

pub async fn list_org_app_suggestions_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: &str,
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

    response
        .json::<Paginated<OrgAppSuggestionDto>>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse org app suggestions response.".to_string(),
        })
}

pub async fn create_org_app_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: &str,
    form: NewOrgAppFormData,
) -> Result<OrgAppDto> {
    let token = ctx.token().expect("Token is required");
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == "new_org_app", CsrfTokenSnafu);

    let url = format!("{}/orgs/{}/apps", &state.config.api_url, org_id);

    let body = NewOrgAppDto {
        app_id: form.app_id,
    };

    let response = state
        .client
        .post(url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to create org app. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "org_apps", Error::OrgAppNotFound).await);
    }

    response
        .json::<OrgAppDto>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse org app response.".to_string(),
        })
}

pub async fn get_org_app_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: &str,
    app_id: &str,
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

    response
        .json::<OrgAppDto>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse org app response.".to_string(),
        })
}

pub async fn delete_org_app_svc(
    state: &AppState,
    ctx: &Ctx,
    org_id: &str,
    app_id: &str,
    csrf_token: &str,
) -> Result<()> {
    let token = ctx.token().expect("Token is required");

    let csrf_result = verify_csrf_token(csrf_token, &state.config.jwt_secret)?;
    ensure!(csrf_result == app_id, CsrfTokenSnafu);

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
