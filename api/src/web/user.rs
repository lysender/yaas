use axum::{
    Extension, Router,
    body::{Body, Bytes},
    extract::{Query, State},
    middleware,
    response::Response,
    routing::{get, post, put},
};
use prost::Message;
use snafu::{OptionExt, ensure};
use validator::Validate;

use yaas::{
    buffed::{
        actor::ActorBuf,
        dto::{
            ChangeCurrentPasswordBuf, NewPasswordBuf, NewUserWithPasswordBuf, PaginatedUsersBuf,
            UpdateUserBuf, UserBuf,
        },
        pagination::PaginatedMetaBuf,
    },
    dto::{
        Actor, ChangeCurrentPasswordDto, ListUsersParamsDto, NewPasswordDto,
        NewUserWithPasswordDto, UpdateUserDto, UserDto,
    },
    role::{Permission, to_buffed_permissions, to_buffed_roles},
    validators::flatten_errors,
};

use crate::{
    Error, Result,
    error::{ForbiddenSnafu, ValidationSnafu, WhateverSnafu},
    services::{
        password::{change_current_password_svc, update_password_svc},
        user::{create_user_svc, delete_user_svc, get_user_svc, list_users_svc, update_user_svc},
    },
    state::AppState,
    web::{build_response, middleware::user_middleware},
};

pub fn users_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_users_handler).post(create_user_handler))
        .nest("/{user_id}", inner_user_routes(state.clone()))
        .with_state(state)
}

fn inner_user_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(get_user_handler)
                .patch(update_user_handler)
                .delete(delete_user_handler),
        )
        .route("/password", put(update_user_password_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            user_middleware,
        ))
        .with_state(state)
}

pub fn current_user_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(profile_handler))
        .route("/authz", get(user_authz_handler))
        .route("/change-password", post(change_password_handler))
        .with_state(state)
}

async fn profile_handler(Extension(actor): Extension<Actor>) -> Result<Response<Body>> {
    let actor = actor.actor.expect("Actor should be present");
    let buffed_user = UserBuf {
        id: actor.user.id,
        email: actor.user.email,
        name: actor.user.name,
        status: actor.user.status,
        created_at: actor.user.created_at,
        updated_at: actor.user.updated_at,
    };

    Ok(build_response(200, buffed_user.encode_to_vec()))
}

async fn user_authz_handler(Extension(actor): Extension<Actor>) -> Result<Response<Body>> {
    let actor = actor.actor.clone();
    let actor = actor.expect("Actor should be present");

    let buffed_actor = ActorBuf {
        id: actor.id,
        org_id: actor.org_id,
        org_count: actor.org_count,
        user: Some(UserBuf {
            id: actor.user.id,
            email: actor.user.email,
            name: actor.user.name,
            status: actor.user.status,
            created_at: actor.user.created_at,
            updated_at: actor.user.updated_at,
        }),
        roles: to_buffed_roles(&actor.roles),
        permissions: to_buffed_permissions(&actor.permissions),
        scope: actor.scope,
    };

    Ok(build_response(200, buffed_actor.encode_to_vec()))
}

async fn change_password_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    body: Bytes,
) -> Result<Response<Body>> {
    let actor = actor.actor.clone();
    let actor = actor.expect("Actor should be present");

    // Parse body as protobuf message
    let Ok(payload) = ChangeCurrentPasswordBuf::decode(body) else {
        return Err(Error::BadProtobuf);
    };

    let data: ChangeCurrentPasswordDto = payload.into();
    let errors = data.validate();

    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let _ = change_current_password_svc(&state, actor.user.id, data).await?;

    Ok(build_response(204, Vec::new()))
}

async fn list_users_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    query: Query<ListUsersParamsDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::UsersList];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let errors = query.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let users = list_users_svc(&state, query.0).await?;
    let buffed_meta = PaginatedMetaBuf {
        page: users.meta.page,
        per_page: users.meta.per_page,
        total_records: users.meta.total_records,
        total_pages: users.meta.total_pages,
    };
    let buffed_list: Vec<UserBuf> = users
        .data
        .into_iter()
        .map(|user| UserBuf {
            id: user.id,
            email: user.email,
            name: user.name,
            status: user.status,
            created_at: user.created_at,
            updated_at: user.updated_at,
        })
        .collect();

    let buffed_result = PaginatedUsersBuf {
        meta: Some(buffed_meta),
        data: buffed_list,
    };

    Ok(build_response(200, buffed_result.encode_to_vec()))
}

async fn create_user_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    body: Bytes,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::UsersCreate];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    // Parse body as protobuf message
    let Ok(payload) = NewUserWithPasswordBuf::decode(body) else {
        return Err(Error::BadProtobuf);
    };

    let data: NewUserWithPasswordDto = payload.into();
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let user = create_user_svc(&state, data).await?;

    let buffed_user = UserBuf {
        id: user.id,
        email: user.email,
        name: user.name,
        status: user.status,
        created_at: user.created_at,
        updated_at: user.updated_at,
    };

    Ok(build_response(201, buffed_user.encode_to_vec()))
}

async fn get_user_handler(user: Extension<UserDto>) -> Result<Response<Body>> {
    let buffed_user = UserBuf {
        id: user.id,
        email: user.email.clone(),
        name: user.name.clone(),
        status: user.status.clone(),
        created_at: user.created_at.clone(),
        updated_at: user.updated_at.clone(),
    };

    Ok(build_response(200, buffed_user.encode_to_vec()))
}

async fn update_user_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    user: Extension<UserDto>,
    body: Bytes,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::UsersEdit];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let actor = actor.actor.clone();
    let actor = actor.expect("Actor should be present");

    // Do not allow updating your own user
    ensure!(
        actor.user.id != user.id,
        ForbiddenSnafu {
            msg: "Updating your own user account not allowed"
        }
    );

    // Parse body as protobuf message
    let Ok(payload) = UpdateUserBuf::decode(body) else {
        return Err(Error::BadProtobuf);
    };

    let data: UpdateUserDto = payload.into();
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let _ = update_user_svc(&state, user.id, data).await?;

    // Not ideal but we need to re-query to get the updated data
    let updated_user = get_user_svc(&state, user.id).await?;
    let updated_user = updated_user.context(WhateverSnafu {
        msg: "Unable to re-query user information.",
    })?;

    let buffed_user = UserBuf {
        id: updated_user.id,
        email: updated_user.email,
        name: updated_user.name,
        status: updated_user.status,
        created_at: updated_user.created_at,
        updated_at: updated_user.updated_at,
    };

    Ok(build_response(200, buffed_user.encode_to_vec()))
}

async fn update_user_password_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    user: Extension<UserDto>,
    body: Bytes,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::UsersEdit];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let actor = actor.actor.clone();
    let actor = actor.expect("Actor should be present");

    // Do not allow updating your own user password
    ensure!(
        actor.user.id != user.id,
        ForbiddenSnafu {
            msg: "Updating your own user password not allowed, use profile change-password endpoint"
        }
    );

    // Parse body as protobuf message
    let Ok(payload) = NewPasswordBuf::decode(body) else {
        return Err(Error::BadProtobuf);
    };

    let data: NewPasswordDto = payload.into();
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let _ = update_password_svc(&state, user.id, data).await?;

    Ok(build_response(204, Vec::new()))
}

async fn delete_user_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    user: Extension<UserDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::UsersDelete];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    // Do not allow deleting your own
    let actor = actor.actor.clone();
    let actor = actor.expect("Actor should be present");

    ensure!(
        actor.user.id != user.id,
        ForbiddenSnafu {
            msg: "Deleting your own user account not allowed"
        }
    );

    let _ = delete_user_svc(&state, user.id).await?;

    Ok(build_response(204, Vec::new()))
}
