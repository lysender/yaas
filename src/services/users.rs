use serde::{Deserialize, Serialize};
use snafu::{ResultExt, ensure};

use crate::ctx::Ctx;
use crate::dto::Paginated;
use crate::dto::{
    ChangeCurrentPasswordDto, ListOrgMembersParamsDto, ListUsersParamsDto, NewPasswordDto,
    NewUserWithPasswordDto, OrgMembershipDto, UpdateUserDto, UserDto,
};
use crate::error::{CsrfTokenSnafu, HttpClientSnafu, HttpResponseParseSnafu, ValidationSnafu};
use crate::run::AppState;
use crate::services::token::verify_csrf_token;
use crate::{Error, Result};

use super::handle_response_error;

#[derive(Clone, Deserialize, Serialize)]
pub struct NewUserFormData {
    pub name: String,
    pub email: String,
    pub password: String,
    pub confirm_password: String,
    pub token: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UserActiveFormData {
    pub token: String,
    pub active: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChangeCurrentPasswordFormData {
    pub token: String,
    pub current_password: String,
    pub new_password: String,
    pub confirm_new_password: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChangePasswordFormData {
    pub token: String,
    pub password: String,
    pub confirm_password: String,
}

pub async fn list_users_svc(
    state: &AppState,
    ctx: &Ctx,
    params: ListUsersParamsDto,
) -> Result<Paginated<UserDto>> {
    let token = ctx.token().expect("Token is required");
    let url = format!("{}/users", &state.config.api_url);

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
            msg: "Unable to list users. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "users", Error::UserNotFound).await);
    }

    response
        .json::<Paginated<UserDto>>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse users listing response.".to_string(),
        })
}

pub async fn create_user_svc(
    state: &AppState,
    ctx: &Ctx,
    form: NewUserFormData,
) -> Result<UserDto> {
    let token = ctx.token().expect("Token is required");
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == "new_user", CsrfTokenSnafu);

    ensure!(
        form.password == form.confirm_password,
        ValidationSnafu {
            msg: "Passwords must match".to_string()
        }
    );

    let url = format!("{}/users", &state.config.api_url);

    let body = NewUserWithPasswordDto {
        name: form.name,
        email: form.email,
        password: form.password,
    };

    let response = state
        .client
        .post(url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to create user. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "users", Error::UserNotFound).await);
    }

    response
        .json::<UserDto>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse user response.".to_string(),
        })
}

pub async fn get_user_svc(state: &AppState, ctx: &Ctx, user_id: &str) -> Result<UserDto> {
    let token = ctx.token().expect("Token is required");
    let url = format!("{}/users/{}", &state.config.api_url, user_id);

    let response = state
        .client
        .get(url)
        .bearer_auth(token)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to get user. Try again later.",
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "users", Error::UserNotFound).await);
    }

    response
        .json::<UserDto>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse user response.".to_string(),
        })
}

pub async fn update_user_status_svc(
    state: &AppState,
    ctx: &Ctx,
    user_id: &str,
    form: UserActiveFormData,
) -> Result<UserDto> {
    let token = ctx.token().expect("Token is required");
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == user_id, CsrfTokenSnafu);

    let url = format!("{}/users/{}", &state.config.api_url, user_id);
    let body = UpdateUserDto {
        name: None,
        status: match form.active {
            Some(_) => Some("active".to_string()),
            None => Some("inactive".to_string()),
        },
    };
    let response = state
        .client
        .patch(url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to update user. Try again later.",
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "users", Error::UserNotFound).await);
    }

    response
        .json::<UserDto>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse user response.".to_string(),
        })
}

pub async fn change_user_current_password_svc(
    state: &AppState,
    ctx: &Ctx,
    user_id: &str,
    form: ChangeCurrentPasswordFormData,
) -> Result<()> {
    let token = ctx.token().expect("Token is required");
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == user_id, CsrfTokenSnafu);

    ensure!(
        form.new_password == form.confirm_new_password,
        ValidationSnafu {
            msg: "Passwords must match."
        }
    );

    let url = format!("{}/user/change-password", &state.config.api_url);

    let body = ChangeCurrentPasswordDto {
        current_password: form.current_password,
        new_password: form.new_password,
    };

    let response = state
        .client
        .post(url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to update user password. Try again later.",
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "users", Error::UserNotFound).await);
    }

    Ok(())
}

pub async fn change_user_password_svc(
    state: &AppState,
    ctx: &Ctx,
    user_id: &str,
    form: ChangePasswordFormData,
) -> Result<()> {
    let token = ctx.token().expect("Token is required");
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == user_id, CsrfTokenSnafu);

    ensure!(
        form.password == form.confirm_password,
        ValidationSnafu {
            msg: "Passwords must match."
        }
    );

    let url = format!("{}/users/{}/password", &state.config.api_url, user_id);

    let body = NewPasswordDto {
        password: form.password,
    };

    let response = state
        .client
        .put(url)
        .bearer_auth(token)
        .json(&body)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to update user password. Try again later.",
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "users", Error::UserNotFound).await);
    }

    Ok(())
}

pub async fn list_org_memberships_svc(
    state: &AppState,
    ctx: &Ctx,
    params: ListOrgMembersParamsDto,
) -> Result<Paginated<OrgMembershipDto>> {
    let token = ctx.token().expect("Token is required");
    let url = format!("{}/user/orgs", &state.config.api_url);

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
            msg: "Unable to list org memberships. Try again later.",
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "users", Error::UserNotFound).await);
    }

    response
        .json::<Paginated<OrgMembershipDto>>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse org memberships response.".to_string(),
        })
}

pub async fn delete_user_svc(
    state: &AppState,
    ctx: &Ctx,
    user_id: &str,
    csrf_token: &str,
) -> Result<()> {
    let token = ctx.token().expect("Token is required");

    let csrf_result = verify_csrf_token(csrf_token, &state.config.jwt_secret)?;
    ensure!(csrf_result == user_id, CsrfTokenSnafu);

    let url = format!("{}/users/{}", &state.config.api_url, user_id,);
    let response = state
        .client
        .delete(url)
        .bearer_auth(token)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to delete user. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "users", Error::UserNotFound).await);
    }

    Ok(())
}
