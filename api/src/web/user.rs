use axum::{
    Extension, Router,
    body::Body,
    extract::{Query, State},
    http::StatusCode,
    middleware,
    response::Response,
    routing::{get, post, put},
};
use snafu::{OptionExt, ensure};
use validator::Validate;

use yaas::{
    dto::{
        Actor, ChangeCurrentPasswordDto, ListUsersParamsDto, NewPasswordDto,
        NewUserWithPasswordDto, SwitchAuthContextDto, UpdateUserDto, UserDto,
    },
    pagination::ListingParamsDto,
    role::Permission,
    validators::flatten_errors,
};

use crate::{
    Result,
    auth::switch_auth_context,
    error::{ForbiddenSnafu, ValidationSnafu, WhateverSnafu},
    services::{
        org_member::list_org_memberships_svc,
        password::{change_current_password_svc, update_password_svc},
        user::{create_user_svc, delete_user_svc, get_user_svc, list_users_svc, update_user_svc},
    },
    state::AppState,
    web::{
        empty_response,
        json_input::{JsonPayload, validate_json_payload},
        json_response,
        middleware::user_middleware,
    },
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
        .route("/orgs", get(list_org_memberships_handler))
        .route("/switch-auth-context", post(switch_org_auth_handler))
        .with_state(state)
}

async fn profile_handler(Extension(actor): Extension<Actor>) -> Result<Response<Body>> {
    let actor = actor.actor.expect("Actor should be present");
    Ok(json_response(StatusCode::OK, actor.user))
}

async fn user_authz_handler(Extension(actor): Extension<Actor>) -> Result<Response<Body>> {
    let actor = actor.actor.expect("Actor should be present");
    Ok(json_response(StatusCode::OK, actor))
}

async fn change_password_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    payload: JsonPayload<ChangeCurrentPasswordDto>,
) -> Result<Response<Body>> {
    let actor = actor.actor.clone();
    let actor = actor.expect("Actor should be present");

    let data = validate_json_payload(payload)?;

    let _ = change_current_password_svc(&state, &actor.user.id, data).await?;

    Ok(empty_response(StatusCode::NO_CONTENT))
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

    // Only superuser can list all users
    // For other users, they only see themselves
    if !actor.is_system_admin() {
        let actor = actor.actor.as_ref().expect("Actor should be present");
        let user = actor.user.clone();

        return Ok(json_response(
            StatusCode::OK,
            yaas::pagination::Paginated {
                meta: yaas::pagination::PaginatedMeta {
                    page: 1,
                    per_page: 50,
                    total_records: 1,
                    total_pages: 1,
                },
                data: vec![user],
            },
        ));
    }

    let users = list_users_svc(&state, query.0).await?;
    Ok(json_response(StatusCode::OK, users))
}

async fn create_user_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    payload: JsonPayload<NewUserWithPasswordDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::UsersCreate];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let data = validate_json_payload(payload)?;

    let user = create_user_svc(&state, data).await?;
    Ok(json_response(StatusCode::CREATED, user))
}

async fn get_user_handler(user: Extension<UserDto>) -> Result<Response<Body>> {
    Ok(json_response(StatusCode::OK, user.0))
}

async fn update_user_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    user: Extension<UserDto>,
    payload: JsonPayload<UpdateUserDto>,
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

    let data = validate_json_payload(payload)?;

    let user_id = user.id.clone();
    let _ = update_user_svc(&state, &user_id, data).await?;

    // Not ideal but we need to re-query to get the updated data
    let updated_user = get_user_svc(&state, &user_id).await?;
    let updated_user = updated_user.context(WhateverSnafu {
        msg: "Unable to re-query user information.",
    })?;

    Ok(json_response(StatusCode::OK, updated_user))
}

async fn update_user_password_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    user: Extension<UserDto>,
    payload: JsonPayload<NewPasswordDto>,
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

    let data = validate_json_payload(payload)?;

    let _ = update_password_svc(&state, &user.id, data).await?;

    Ok(empty_response(StatusCode::NO_CONTENT))
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

    let _ = delete_user_svc(&state, &user.id).await?;

    Ok(empty_response(StatusCode::NO_CONTENT))
}

async fn list_org_memberships_handler(
    Extension(actor): Extension<Actor>,
    State(state): State<AppState>,
    Query(query): Query<ListingParamsDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::OrgsList];
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

    let actor = actor.actor.as_ref().expect("Actor should be present");
    let user_id = actor.user.id.clone();

    let memberships = list_org_memberships_svc(&state, &user_id, query).await?;
    Ok(json_response(StatusCode::OK, memberships))
}

pub async fn switch_org_auth_handler(
    Extension(actor): Extension<Actor>,
    State(state): State<AppState>,
    payload: JsonPayload<SwitchAuthContextDto>,
) -> Result<Response<Body>> {
    let data = validate_json_payload(payload)?;

    let auth_res = switch_auth_context(&state, &actor, &data).await?;
    Ok(json_response(StatusCode::OK, auth_res))
}
