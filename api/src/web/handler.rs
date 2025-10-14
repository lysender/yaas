use axum::{
    Extension,
    body::{Body, Bytes},
    extract::{Json, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use prost::Message;
use serde::Serialize;
use snafu::{OptionExt, ensure};
use validator::Validate;

use yaas::{
    actor::{Actor, Credentials},
    buffed::{
        actor::{ActorBuf, AuthResponseBuf, CredentialsBuf},
        dto::{
            AppBuf, ChangeCurrentPasswordBuf, ErrorMessageBuf, NewAppBuf, NewOrgAppBuf, NewOrgBuf,
            NewOrgMemberBuf, NewPasswordBuf, NewUserWithPasswordBuf, OrgAppBuf, OrgBuf,
            OrgMemberBuf, OrgMemberSuggestionBuf, OrgMembershipBuf, PaginatedAppsBuf,
            PaginatedOrgAppsBuf, PaginatedOrgMembersBuf, PaginatedOrgMembershipsBuf,
            PaginatedOrgsBuf, PaginatedUsersBuf, SetupBodyBuf, SuperuserBuf, UpdateAppBuf,
            UpdateOrgBuf, UpdateOrgMemberBuf, UpdateUserBuf, UserBuf,
        },
        pagination::PaginatedMetaBuf,
    },
    dto::{
        AppDto, ChangeCurrentPasswordDto, ListAppsParamsDto, ListOrgAppsParamsDto,
        ListOrgMembersParamsDto, ListOrgsParamsDto, ListUsersParamsDto, NewAppDto, NewOrgAppDto,
        NewOrgDto, NewOrgMemberDto, NewPasswordDto, NewUserWithPasswordDto, OrgAppDto, OrgDto,
        OrgMemberDto, SetupBodyDto, UpdateAppDto, UpdateOrgDto, UpdateOrgMemberDto, UpdateUserDto,
        UserDto,
    },
    role::{Permission, to_buffed_permissions, to_buffed_roles},
    validators::flatten_errors,
};

use crate::{
    Error, Result,
    auth::authenticate,
    error::{ForbiddenSnafu, ValidationSnafu, WhateverSnafu},
    health::{check_liveness, check_readiness},
    services::{
        app::{
            create_app_svc, delete_app_svc, get_app_svc, list_apps_svc, regenerate_app_secret_svc,
            update_app_svc,
        },
        org::{create_org_svc, delete_org_svc, get_org_svc, list_orgs_svc, update_org_svc},
        org_app::{create_org_app_svc, delete_org_app_svc, list_org_apps_svc},
        org_member::{
            create_org_member_svc, delete_org_member_svc, get_org_member_svc,
            list_org_member_suggestions_svc, list_org_members_svc, update_org_member_svc,
        },
        password::{change_current_password_svc, update_password_svc},
        superuser::setup_superuser_svc,
        user::{create_user_svc, delete_user_svc, get_user_svc, list_users_svc, update_user_svc},
    },
    state::AppState,
    web::response::JsonResponse,
};

#[derive(Serialize)]
pub struct AppMeta {
    pub name: String,
    pub version: String,
}

pub async fn home_handler() -> impl IntoResponse {
    Json(AppMeta {
        name: "yaas".to_string(),
        version: "0.1.0".to_string(),
    })
}

pub async fn not_found_handler(State(_state): State<AppState>) -> Result<Response<Body>> {
    let error_message = ErrorMessageBuf {
        status_code: StatusCode::NOT_FOUND.as_u16() as u32,
        message: "Not Found".to_string(),
        error: "Not Found".to_string(),
        error_code: None,
    };

    Ok(Response::builder()
        .status(404)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(error_message.encode_to_vec()))
        .unwrap())
}

pub async fn health_live_handler() -> Result<JsonResponse> {
    let health = check_liveness().await?;
    Ok(JsonResponse::new(serde_json::to_string(&health).unwrap()))
}

pub async fn health_ready_handler(State(state): State<AppState>) -> Result<JsonResponse> {
    let health = check_readiness(state.db).await?;
    let status = if health.is_healthy() {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    Ok(JsonResponse::with_status(
        status,
        serde_json::to_string(&health).unwrap(),
    ))
}

fn build_response(status_code: u16, body: Vec<u8>) -> Response<Body> {
    Response::builder()
        .status(status_code)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(body))
        .unwrap()
}

pub async fn authenticate_handler(
    State(state): State<AppState>,
    body: Bytes,
) -> Result<Response<Body>> {
    // Parse body as protobuf message
    let Ok(creds) = CredentialsBuf::decode(body) else {
        return Err(Error::BadProtobuf);
    };

    let credentials = Credentials {
        email: creds.email,
        password: creds.password,
    };

    let errors = credentials.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let auth_res = authenticate(&state, &credentials).await?;
    let buffed_auth_res = AuthResponseBuf {
        user: Some(UserBuf {
            id: auth_res.user.id,
            email: auth_res.user.email,
            name: auth_res.user.name,
            status: auth_res.user.status,
            created_at: auth_res.user.created_at,
            updated_at: auth_res.user.updated_at,
        }),
        token: auth_res.token,
        select_org_token: auth_res.select_org_token,
        select_org_options: auth_res
            .select_org_options
            .into_iter()
            .map(|m| OrgMembershipBuf {
                org_id: m.org_id,
                org_name: m.org_name,
                user_id: m.user_id,
                roles: to_buffed_roles(&m.roles),
            })
            .collect(),
    };

    Ok(build_response(200, buffed_auth_res.encode_to_vec()))
}

pub async fn setup_handler(State(state): State<AppState>, body: Bytes) -> Result<Response<Body>> {
    // Parse body as protobuf message
    let Ok(payload) = SetupBodyBuf::decode(body) else {
        return Err(Error::BadProtobuf);
    };

    let payload = SetupBodyDto {
        setup_key: payload.setup_key,
        email: payload.email,
        password: payload.password,
    };

    let errors = payload.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let superuser = setup_superuser_svc(&state, payload).await?;
    let buffed_superuser = SuperuserBuf {
        id: superuser.id,
        created_at: superuser.created_at,
    };

    Ok(build_response(200, buffed_superuser.encode_to_vec()))
}

pub async fn profile_handler(Extension(actor): Extension<Actor>) -> Result<Response<Body>> {
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

pub async fn user_authz_handler(Extension(actor): Extension<Actor>) -> Result<Response<Body>> {
    let actor = actor.actor.clone();
    let actor = actor.expect("Actor should be present");

    let buffed_actor = ActorBuf {
        id: actor.id,
        org_id: actor.org_id,
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

pub async fn change_password_handler(
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

pub async fn list_users_handler(
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

pub async fn create_user_handler(
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

pub async fn get_user_handler(user: Extension<UserDto>) -> Result<Response<Body>> {
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

pub async fn update_user_handler(
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

pub async fn update_user_password_handler(
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

pub async fn delete_user_handler(
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

pub async fn list_orgs_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    query: Query<ListOrgsParamsDto>,
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

    let orgs = list_orgs_svc(&state, query.0).await?;
    let buffed_meta = PaginatedMetaBuf {
        page: orgs.meta.page,
        per_page: orgs.meta.per_page,
        total_records: orgs.meta.total_records,
        total_pages: orgs.meta.total_pages,
    };
    let buffed_list: Vec<OrgBuf> = orgs
        .data
        .into_iter()
        .map(|org| OrgBuf {
            id: org.id,
            name: org.name,
            status: org.status,
            owner_id: org.owner_id,
            owner_email: org.owner_email,
            owner_name: org.owner_name,
            created_at: org.created_at,
            updated_at: org.updated_at,
        })
        .collect();

    let buffed_result = PaginatedOrgsBuf {
        meta: Some(buffed_meta),
        data: buffed_list,
    };

    Ok(build_response(200, buffed_result.encode_to_vec()))
}

pub async fn create_org_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    body: Bytes,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::OrgsCreate];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    // Parse body as protobuf message
    let Ok(payload) = NewOrgBuf::decode(body) else {
        return Err(Error::BadProtobuf);
    };

    let data: NewOrgDto = payload.into();
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let org = create_org_svc(&state, data).await?;

    let buffed_org = OrgBuf {
        id: org.id,
        name: org.name,
        status: org.status,
        owner_id: org.owner_id,
        owner_email: org.owner_email,
        owner_name: org.owner_name,
        created_at: org.created_at,
        updated_at: org.updated_at,
    };

    Ok(build_response(201, buffed_org.encode_to_vec()))
}

pub async fn get_org_handler(Extension(org): Extension<OrgDto>) -> Result<Response<Body>> {
    let buffed_org = OrgBuf {
        id: org.id,
        name: org.name,
        status: org.status,
        owner_id: org.owner_id,
        owner_email: org.owner_email,
        owner_name: org.owner_name,
        created_at: org.created_at,
        updated_at: org.updated_at,
    };

    Ok(build_response(200, buffed_org.encode_to_vec()))
}

pub async fn update_org_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    org: Extension<OrgDto>,
    body: Bytes,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::OrgsEdit];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    // Do not allow superusers to update their own org
    if actor.is_system_admin() {
        let actor = actor.actor.clone();
        let actor = actor.expect("Actor should be present");

        ensure!(
            actor.org_id != org.id,
            ForbiddenSnafu {
                msg: "Superusers cannot update their own organization"
            }
        );
    }

    // Parse body as protobuf message
    let Ok(payload) = UpdateOrgBuf::decode(body) else {
        return Err(Error::BadProtobuf);
    };

    let data: UpdateOrgDto = payload.into();
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let _ = update_org_svc(&state, org.id, data).await?;

    // Not ideal but we need to re-query to get the updated data
    let updated_org = get_org_svc(&state, org.id).await?;
    let updated_org = updated_org.context(WhateverSnafu {
        msg: "Unable to re-query org information.",
    })?;

    let buffed_org = OrgBuf {
        id: updated_org.id,
        name: updated_org.name,
        status: updated_org.status,
        owner_id: updated_org.owner_id,
        owner_email: updated_org.owner_email,
        owner_name: updated_org.owner_name,
        created_at: updated_org.created_at,
        updated_at: updated_org.updated_at,
    };

    Ok(build_response(200, buffed_org.encode_to_vec()))
}

pub async fn delete_org_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    org: Extension<OrgDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::OrgsDelete];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    // Do not allow deleting your own org
    let actor = actor.actor.clone();
    let actor = actor.expect("Actor should be present");

    ensure!(
        actor.org_id != org.id,
        ForbiddenSnafu {
            msg: "Deleting your own org not allowed"
        }
    );

    let _ = delete_org_svc(&state, org.id).await?;

    Ok(build_response(204, Vec::new()))
}

pub async fn list_apps_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    query: Query<ListAppsParamsDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::AppsList];
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

    let apps = list_apps_svc(&state, query.0).await?;
    let buffed_meta = PaginatedMetaBuf {
        page: apps.meta.page,
        per_page: apps.meta.per_page,
        total_records: apps.meta.total_records,
        total_pages: apps.meta.total_pages,
    };
    let buffed_list: Vec<AppBuf> = apps
        .data
        .into_iter()
        .map(|app| AppBuf {
            id: app.id,
            name: app.name,
            client_id: app.client_id,
            client_secret: app.client_secret,
            redirect_uri: app.redirect_uri,
            created_at: app.created_at,
            updated_at: app.updated_at,
        })
        .collect();

    let buffed_result = PaginatedAppsBuf {
        meta: Some(buffed_meta),
        data: buffed_list,
    };

    Ok(build_response(200, buffed_result.encode_to_vec()))
}

pub async fn create_app_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    body: Bytes,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::AppsCreate];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    // Parse body as protobuf message
    let Ok(payload) = NewAppBuf::decode(body) else {
        return Err(Error::BadProtobuf);
    };

    let data: NewAppDto = payload.into();
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let app = create_app_svc(&state, data).await?;

    let buffed_app = AppBuf {
        id: app.id,
        name: app.name,
        client_id: app.client_id,
        client_secret: app.client_secret,
        redirect_uri: app.redirect_uri,
        created_at: app.created_at,
        updated_at: app.updated_at,
    };

    Ok(build_response(201, buffed_app.encode_to_vec()))
}

pub async fn get_app_handler(app: Extension<AppDto>) -> Result<Response<Body>> {
    let buffed_app = AppBuf {
        id: app.id,
        name: app.name.clone(),
        client_id: app.client_id.clone(),
        client_secret: app.client_secret.clone(),
        redirect_uri: app.redirect_uri.clone(),
        created_at: app.created_at.clone(),
        updated_at: app.updated_at.clone(),
    };

    Ok(build_response(200, buffed_app.encode_to_vec()))
}

pub async fn update_app_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    app: Extension<AppDto>,
    body: Bytes,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::AppsEdit];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    // Parse body as protobuf message
    let Ok(payload) = UpdateAppBuf::decode(body) else {
        return Err(Error::BadProtobuf);
    };

    let data: UpdateAppDto = payload.into();
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let _ = update_app_svc(&state, app.id, data).await?;

    // Not ideal but we need to re-query to get the updated data
    let updated_app = get_app_svc(&state, app.id).await?;
    let updated_app = updated_app.context(WhateverSnafu {
        msg: "Unable to re-query app information.",
    })?;

    let buffed_app = AppBuf {
        id: updated_app.id,
        name: updated_app.name,
        client_id: updated_app.client_id,
        client_secret: updated_app.client_secret,
        redirect_uri: updated_app.redirect_uri,
        created_at: updated_app.created_at,
        updated_at: updated_app.updated_at,
    };

    Ok(build_response(200, buffed_app.encode_to_vec()))
}

pub async fn regenerate_app_secret_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    app: Extension<AppDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::AppsEdit];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let _ = regenerate_app_secret_svc(&state, app.id).await?;

    // Not ideal but we need to re-query to get the updated data
    let updated_app = get_app_svc(&state, app.id).await?;
    let updated_app = updated_app.context(WhateverSnafu {
        msg: "Unable to re-query app information.",
    })?;

    let buffed_app = AppBuf {
        id: updated_app.id,
        name: updated_app.name,
        client_id: updated_app.client_id,
        client_secret: updated_app.client_secret,
        redirect_uri: updated_app.redirect_uri,
        created_at: updated_app.created_at,
        updated_at: updated_app.updated_at,
    };

    Ok(build_response(200, buffed_app.encode_to_vec()))
}

pub async fn delete_app_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    app: Extension<AppDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::AppsDelete];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let _ = delete_app_svc(&state, app.id).await?;

    Ok(build_response(204, Vec::new()))
}

pub async fn list_org_members_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    org: Extension<OrgDto>,
    query: Query<ListOrgMembersParamsDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::OrgMembersList];
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

    let members = list_org_members_svc(&state, org.id, query.0).await?;
    let buffed_meta = PaginatedMetaBuf {
        page: members.meta.page,
        per_page: members.meta.per_page,
        total_records: members.meta.total_records,
        total_pages: members.meta.total_pages,
    };
    let buffed_list: Vec<OrgMemberBuf> = members
        .data
        .into_iter()
        .map(|member| OrgMemberBuf {
            id: member.id,
            org_id: member.org_id,
            user_id: member.user_id,
            member_email: member.member_email,
            member_name: member.member_name,
            roles: to_buffed_roles(&member.roles),
            status: member.status,
            created_at: member.created_at,
            updated_at: member.updated_at,
        })
        .collect();

    let buffed_result = PaginatedOrgMembersBuf {
        meta: Some(buffed_meta),
        data: buffed_list,
    };

    Ok(build_response(200, buffed_result.encode_to_vec()))
}

pub async fn list_org_member_suggestions_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Extension(org): Extension<OrgDto>,
    Query(query): Query<ListOrgMembersParamsDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::OrgMembersList];
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

    let members = list_org_member_suggestions_svc(&state, org.id, query).await?;
    let buffed_meta = PaginatedMetaBuf {
        page: members.meta.page,
        per_page: members.meta.per_page,
        total_records: members.meta.total_records,
        total_pages: members.meta.total_pages,
    };
    let buffed_list: Vec<OrgMemberSuggestionBuf> = members
        .data
        .into_iter()
        .map(|member| OrgMemberSuggestionBuf {
            id: member.id,
            email: member.email,
            name: member.name,
        })
        .collect();

    let buffed_result = PaginatedOrgMembershipsBuf {
        meta: Some(buffed_meta),
        data: buffed_list,
    };

    Ok(build_response(200, buffed_result.encode_to_vec()))
}

pub async fn create_org_member_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    org: Extension<OrgDto>,
    body: Bytes,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::OrgMembersCreate];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    // Parse body as protobuf message
    let Ok(payload) = NewOrgMemberBuf::decode(body) else {
        return Err(Error::BadProtobuf);
    };

    let data: NewOrgMemberDto = payload.try_into()?;
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let member = create_org_member_svc(&state, org.id, data).await?;

    // Not ideal but we need to re-query to get the full member details
    let member = get_org_member_svc(&state, org.id, member.user_id).await?;
    let member = member.context(WhateverSnafu {
        msg: "Unable to re-query org member information.",
    })?;

    let buffed_member = OrgMemberBuf {
        id: member.id,
        org_id: member.org_id,
        user_id: member.user_id,
        member_email: member.member_email,
        member_name: member.member_name,
        roles: to_buffed_roles(&member.roles),
        status: member.status,
        created_at: member.created_at,
        updated_at: member.updated_at,
    };

    Ok(build_response(201, buffed_member.encode_to_vec()))
}

pub async fn get_org_member_handler(member: Extension<OrgMemberDto>) -> Result<Response<Body>> {
    let buffed_member = OrgMemberBuf {
        id: member.id,
        org_id: member.org_id,
        user_id: member.user_id,
        member_email: member.member_email.clone(),
        member_name: member.member_name.clone(),
        roles: to_buffed_roles(&member.roles),
        status: member.status.clone(),
        created_at: member.created_at.clone(),
        updated_at: member.updated_at.clone(),
    };

    Ok(build_response(200, buffed_member.encode_to_vec()))
}

pub async fn update_org_member_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    member: Extension<OrgMemberDto>,
    body: Bytes,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::OrgMembersEdit];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    // Do not allow updating your own within the org
    let actor = actor.actor.clone();
    let actor = actor.expect("Actor should be present");

    if actor.org_id == member.org_id {
        ensure!(
            actor.user.id != member.user_id,
            ForbiddenSnafu {
                msg: "Updating yourself within the organization is not allowed"
            }
        );
    }

    // Parse body as protobuf message
    let Ok(payload) = UpdateOrgMemberBuf::decode(body) else {
        return Err(Error::BadProtobuf);
    };

    let data: UpdateOrgMemberDto = payload.try_into()?;
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let _ = update_org_member_svc(&state, member.id, data).await?;

    // Not ideal but we need to re-query to get the updated data
    let updated_member = get_org_member_svc(&state, member.org_id, member.user_id).await?;
    let updated_member = updated_member.context(WhateverSnafu {
        msg: "Unable to re-query org member information.",
    })?;

    let buffed_member = OrgMemberBuf {
        id: updated_member.id,
        org_id: updated_member.org_id,
        user_id: updated_member.user_id,
        member_email: updated_member.member_email,
        member_name: updated_member.member_name,
        roles: to_buffed_roles(&updated_member.roles),
        status: updated_member.status,
        created_at: updated_member.created_at,
        updated_at: updated_member.updated_at,
    };

    Ok(build_response(200, buffed_member.encode_to_vec()))
}

pub async fn delete_org_member_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    member: Extension<OrgMemberDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::OrgMembersDelete];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    // Do not allow deleting your own from the org
    let actor = actor.actor.clone();
    let actor = actor.expect("Actor should be present");

    if actor.org_id == member.org_id {
        ensure!(
            actor.user.id != member.user_id,
            ForbiddenSnafu {
                msg: "Deleting yourself from the organization is not allowed"
            }
        );
    }

    let _ = delete_org_member_svc(&state, member.id).await?;

    Ok(build_response(204, Vec::new()))
}

pub async fn list_org_apps_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    org: Extension<OrgDto>,
    query: Query<ListOrgAppsParamsDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::OrgAppsList];
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

    let org_apps = list_org_apps_svc(&state, org.id, query.0).await?;
    let buffed_meta = PaginatedMetaBuf {
        page: org_apps.meta.page,
        per_page: org_apps.meta.per_page,
        total_records: org_apps.meta.total_records,
        total_pages: org_apps.meta.total_pages,
    };
    let buffed_list: Vec<OrgAppBuf> = org_apps
        .data
        .into_iter()
        .map(|org_app| OrgAppBuf {
            id: org_app.id,
            org_id: org_app.org_id,
            app_id: org_app.app_id,
            app_name: org_app.app_name,
            created_at: org_app.created_at,
        })
        .collect();

    let buffed_result = PaginatedOrgAppsBuf {
        meta: Some(buffed_meta),
        data: buffed_list,
    };

    Ok(build_response(200, buffed_result.encode_to_vec()))
}

pub async fn create_org_app_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    org: Extension<OrgDto>,
    body: Bytes,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::OrgAppsCreate];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    // Parse body as protobuf message
    let Ok(payload) = NewOrgAppBuf::decode(body) else {
        return Err(Error::BadProtobuf);
    };

    let data: NewOrgAppDto = payload.into();
    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let mut org_app = create_org_app_svc(&state, org.id, data).await?;

    // We need to fetch the app name from the app service
    let app = get_app_svc(&state, org_app.app_id).await?;
    let app = app.context(WhateverSnafu {
        msg: "Unable to fetch app information for org app.",
    })?;

    org_app.app_name = Some(app.name);

    let buffed_org_app = OrgAppBuf {
        id: org_app.id,
        org_id: org_app.org_id,
        app_id: org_app.app_id,
        app_name: org_app.app_name,
        created_at: org_app.created_at,
    };

    Ok(build_response(201, buffed_org_app.encode_to_vec()))
}

pub async fn get_org_app_handler(
    state: State<AppState>,
    org_app: Extension<OrgAppDto>,
) -> Result<Response<Body>> {
    // We need to fetch the app name from the app service
    let app = get_app_svc(&state, org_app.app_id).await?;
    let app = app.context(WhateverSnafu {
        msg: "Unable to fetch app information for org app.",
    })?;

    let buffed_org_app = OrgAppBuf {
        id: org_app.id,
        org_id: org_app.org_id,
        app_id: org_app.app_id,
        app_name: Some(app.name),
        created_at: org_app.created_at.clone(),
    };

    Ok(build_response(200, buffed_org_app.encode_to_vec()))
}

pub async fn delete_org_app_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    org_app: Extension<OrgAppDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::OrgAppsDelete];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let _ = delete_org_app_svc(&state, org_app.id).await?;

    Ok(build_response(204, Vec::new()))
}
