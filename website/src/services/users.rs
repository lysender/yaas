use prost::Message;
use serde::{Deserialize, Serialize};
use snafu::{OptionExt, ResultExt, ensure};
use yaas::buffed::dto::{
    ChangeCurrentPasswordBuf, NewUserWithPasswordBuf, PaginatedUsersBuf, UpdateUserBuf, UserBuf,
};
use yaas::pagination::{Paginated, PaginatedMeta};

use crate::ctx::Ctx;
use crate::error::{
    CsrfTokenSnafu, HttpClientSnafu, HttpResponseBytesSnafu, HttpResponseParseSnafu,
    ProtobufDecodeSnafu, ValidationSnafu, WhateverSnafu,
};
use crate::run::AppState;
use crate::services::token::verify_csrf_token;
use crate::{Error, Result};
use yaas::dto::{ListUsersParamsDto, UserDto};

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
pub struct ResetPasswordFormData {
    pub token: String,
    pub password: String,
    pub confirm_password: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ResetPasswordData {
    pub password: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ChangePasswordFormData {
    pub token: String,
    pub current_password: String,
    pub new_password: String,
    pub confirm_new_password: String,
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

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let listing = PaginatedUsersBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;

    // Convert listing to dto
    let meta: PaginatedMeta = listing
        .meta
        .context(WhateverSnafu {
            msg: "Missing pagination metadata.".to_string(),
        })?
        .into();

    let users: Vec<UserDto> = listing.data.into_iter().map(|u| u.into()).collect();
    let dto: Paginated<UserDto> = Paginated { meta, data: users };

    Ok(dto)
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
        &form.password == &form.confirm_password,
        ValidationSnafu {
            msg: "Passwords must match".to_string()
        }
    );

    let url = format!("{}/users", &state.config.api_url);

    let body = NewUserWithPasswordBuf {
        name: form.name,
        email: form.email,
        password: form.password,
    };

    let response = state
        .client
        .post(url)
        .bearer_auth(token)
        .body(prost::Message::encode_to_vec(&body))
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to create user. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "users", Error::UserNotFound).await);
    }

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let user = UserBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;
    let dto: UserDto = user.into();

    Ok(dto)
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

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let user = UserBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;
    let dto: UserDto = user.into();

    Ok(dto)
}

pub async fn update_user_status_svc(
    state: &AppState,
    ctx: &Ctx,
    user_id: i32,
    form: UserActiveFormData,
) -> Result<UserDto> {
    let token = ctx.token().expect("Token is required");
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(&csrf_result == &user_id.to_string(), CsrfTokenSnafu);

    let url = format!("{}/users/{}", &state.config.api_url, user_id);
    let body = UpdateUserBuf {
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
        .body(prost::Message::encode_to_vec(&body))
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to update user. Try again later.",
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "users", Error::UserNotFound).await);
    }

    let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
    let user = UserBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;
    let dto: UserDto = user.into();

    Ok(dto)
}

pub async fn reset_user_password_svc(
    state: &AppState,
    ctx: &Ctx,
    user_id: i32,
    form: ResetPasswordFormData,
) -> Result<UserDto> {
    let token = ctx.token().expect("Token is required");
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(&csrf_result == &user_id.to_string(), CsrfTokenSnafu);

    ensure!(
        &form.password == &form.confirm_password,
        ValidationSnafu {
            msg: "Passwords must match."
        }
    );

    let url = format!("{}/users/{}/reset-password", &state.config.api_url, user_id);

    let data = ResetPasswordData {
        password: form.password.clone(),
    };

    let response = state
        .client
        .post(url)
        .bearer_auth(token)
        .json(&data)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to update user password. Try again later.",
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "users", Error::UserNotFound).await);
    }

    let user = response
        .json::<UserDto>()
        .await
        .context(HttpResponseParseSnafu {
            msg: "Unable to parse user information.",
        })?;

    Ok(user)
}

pub async fn change_user_password_svc(
    state: &AppState,
    ctx: &Ctx,
    user_id: i32,
    form: ChangePasswordFormData,
) -> Result<()> {
    let token = ctx.token().expect("Token is required");
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(&csrf_result == &user_id.to_string(), CsrfTokenSnafu);

    ensure!(
        &form.new_password == &form.confirm_new_password,
        ValidationSnafu {
            msg: "Passwords must match."
        }
    );

    let url = format!("{}/user/change-password", &state.config.api_url);

    let body = ChangeCurrentPasswordBuf {
        current_password: form.current_password,
        new_password: form.new_password,
    };

    let response = state
        .client
        .post(url)
        .bearer_auth(token)
        .body(prost::Message::encode_to_vec(&body))
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

pub async fn delete_user_svc(
    state: &AppState,
    ctx: &Ctx,
    user_id: i32,
    csrf_token: &str,
) -> Result<()> {
    let token = ctx.token().expect("Token is required");

    let csrf_result = verify_csrf_token(&csrf_token, &state.config.jwt_secret)?;
    ensure!(&csrf_result == &user_id.to_string(), CsrfTokenSnafu);

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
