use axum::{
    Extension,
    body::{Body, Bytes},
    extract::{Json, Multipart, Path, Query, State, rejection::JsonRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use core::result::Result as CoreResult;
use prost::Message;
use serde::Serialize;
use snafu::{OptionExt, ResultExt, ensure};
use tokio::{fs::File, fs::create_dir_all, io::AsyncWriteExt};
use validator::Validate;

use yaas::{
    actor::{Actor, Credentials},
    buffed::{
        actor::{ActorBuf, AuthResponseBuf, CredentialsBuf},
        dto::{
            AppBuf, ChangeCurrentPasswordBuf, ErrorMessageBuf, NewAppBuf, NewOrgBuf,
            NewOrgMemberBuf, NewUserBuf, NewUserWithPasswordBuf, OrgBuf, OrgMemberBuf,
            OrgMembershipBuf, PaginatedAppsBuf, PaginatedOrgMembersBuf, PaginatedOrgsBuf,
            PaginatedUsersBuf, SetupBodyBuf, SuperuserBuf, UpdateAppBuf, UpdateOrgBuf,
            UpdateOrgMemberBuf, UpdateUserBuf, UserBuf,
        },
        pagination::PaginatedMetaBuf,
    },
    dto::{
        AppDto, ChangeCurrentPasswordDto, ErrorMessageDto, ListAppsParamsDto,
        ListOrgMembersParamsDto, ListOrgsParamsDto, ListUsersParamsDto, NewAppDto, NewOrgDto,
        NewOrgMemberDto, NewUserDto, NewUserWithPasswordDto, OrgDto, OrgMemberDto, SetupBodyDto,
        UpdateAppDto, UpdateOrgDto, UpdateUserDto, UserDto,
    },
    role::{buffed_to_roles, to_buffed_permissions, to_buffed_roles},
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
        org_member::{create_org_member_svc, list_org_members_svc},
        password::change_current_password_svc,
        superuser::setup_superuser_svc,
        user::{create_user_svc, delete_user_svc, get_user_svc, list_users_svc, update_user_svc},
    },
    state::AppState,
    web::response::JsonResponse,
};

// use crate::{
//     auth::{
//         authenticate,
//         user::{
//             change_current_password, create_user, update_password, update_user_roles,
//             update_user_status,
//         },
//     },
//     bucket::{create_bucket, delete_bucket, update_bucket},
//     client::{create_client, delete_client, update_client},
//     dir::{create_dir, delete_dir, update_dir},
//     error::{
//         CreateFileSnafu, DbSnafu, ErrorResponse, ForbiddenSnafu, JsonRejectionSnafu,
//         MissingUploadFileSnafu, Result, StorageSnafu, UploadDirSnafu, WhateverSnafu,
//     },
//     file::create_file,
//     health::{check_liveness, check_readiness},
//     state::AppState,
//     web::{params::Params, response::JsonResponse},
// };
// use db::bucket::{NewBucket, UpdateBucket};
// use db::client::{ClientDefaultBucket, NewClient, UpdateClient};
// use db::dir::{ListDirsParams, NewDir, UpdateDir};
// use db::file::{FileObject, FilePayload, ListFilesParams};
// use db::user::{
//     ChangeCurrentPassword, NewUser, UpdateUserPassword, UpdateUserRoles, UpdateUserStatus,
// };
// use yaas::{
//     actor::{Actor, Credentials},
//     bucket::BucketDto,
//     client::ClientDto,
//     dir::DirDto,
//     file::{FileDto, ImgVersion},
//     pagination::Paginated,
//     role::Permission,
//     user::UserDto,
//     utils::slugify_prefixed,
// };

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
    let health = check_readiness(&state.config, state.db).await?;
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

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(buffed_auth_res.encode_to_vec()))
        .unwrap())
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

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(buffed_superuser.encode_to_vec()))
        .unwrap())
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

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(buffed_user.encode_to_vec()))
        .unwrap())
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

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(buffed_actor.encode_to_vec()))
        .unwrap())
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

    Ok(Response::builder()
        .status(204)
        .header("Content-Type", "application/octet-stream")
        .body(Body::from(""))
        .unwrap())
}

pub async fn list_orgs_handler(
    state: State<AppState>,
    query: Query<ListOrgsParamsDto>,
) -> Result<Response<Body>> {
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
            created_at: org.created_at,
            updated_at: org.updated_at,
        })
        .collect();

    let buffed_result = PaginatedOrgsBuf {
        meta: Some(buffed_meta),
        data: buffed_list,
    };

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(buffed_result.encode_to_vec()))
        .unwrap())
}

pub async fn create_org_handler(state: State<AppState>, body: Bytes) -> Result<Response<Body>> {
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
        created_at: org.created_at,
        updated_at: org.updated_at,
    };

    Ok(Response::builder()
        .status(201)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(buffed_org.encode_to_vec()))
        .unwrap())
}

pub async fn get_org_handler(org: Extension<OrgDto>) -> Result<Response<Body>> {
    let buffed_org = OrgBuf {
        id: org.id,
        name: org.name.clone(),
        status: org.status.clone(),
        owner_id: org.owner_id.clone(),
        created_at: org.created_at.clone(),
        updated_at: org.updated_at.clone(),
    };

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(buffed_org.encode_to_vec()))
        .unwrap())
}

pub async fn update_org_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    org: Extension<OrgDto>,
    body: Bytes,
) -> Result<Response<Body>> {
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
        created_at: updated_org.created_at,
        updated_at: updated_org.updated_at,
    };

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(buffed_org.encode_to_vec()))
        .unwrap())
}

pub async fn delete_org_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    org: Extension<OrgDto>,
) -> Result<Response<Body>> {
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

    Ok(Response::builder()
        .status(204)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(vec![]))
        .unwrap())
}

pub async fn list_apps_handler(
    state: State<AppState>,
    query: Query<ListAppsParamsDto>,
) -> Result<Response<Body>> {
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

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(buffed_result.encode_to_vec()))
        .unwrap())
}

pub async fn create_app_handler(state: State<AppState>, body: Bytes) -> Result<Response<Body>> {
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

    Ok(Response::builder()
        .status(201)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(buffed_app.encode_to_vec()))
        .unwrap())
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

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(buffed_app.encode_to_vec()))
        .unwrap())
}

pub async fn update_app_handler(
    state: State<AppState>,
    app: Extension<AppDto>,
    body: Bytes,
) -> Result<Response<Body>> {
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

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(buffed_app.encode_to_vec()))
        .unwrap())
}

pub async fn regenerate_app_secret_handler(
    state: State<AppState>,
    app: Extension<AppDto>,
) -> Result<Response<Body>> {
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

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(buffed_app.encode_to_vec()))
        .unwrap())
}

pub async fn delete_app_handler(
    state: State<AppState>,
    app: Extension<AppDto>,
) -> Result<Response<Body>> {
    let _ = delete_app_svc(&state, app.id).await?;

    Ok(Response::builder()
        .status(204)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(vec![]))
        .unwrap())
}

pub async fn list_org_members_handler(
    state: State<AppState>,
    org: Extension<OrgDto>,
    query: Query<ListOrgMembersParamsDto>,
) -> Result<Response<Body>> {
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
            name: member.name,
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

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(buffed_result.encode_to_vec()))
        .unwrap())
}

pub async fn create_org_member_handler(
    state: State<AppState>,
    org: Extension<OrgDto>,
    body: Bytes,
) -> Result<Response<Body>> {
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

    let buffed_member = OrgMemberBuf {
        id: member.id,
        org_id: member.org_id,
        user_id: member.user_id,
        name: member.name,
        roles: to_buffed_roles(&member.roles),
        status: member.status,
        created_at: member.created_at,
        updated_at: member.updated_at,
    };

    Ok(Response::builder()
        .status(201)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(buffed_member.encode_to_vec()))
        .unwrap())
}

pub async fn get_org_member_handler(member: Extension<OrgMemberDto>) -> Result<Response<Body>> {
    let buffed_member = OrgMemberBuf {
        id: member.id,
        org_id: member.org_id,
        user_id: member.user_id,
        name: member.name.clone(),
        roles: to_buffed_roles(&member.roles),
        status: member.status.clone(),
        created_at: member.created_at.clone(),
        updated_at: member.updated_at.clone(),
    };

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(buffed_member.encode_to_vec()))
        .unwrap())
}

// pub async fn get_bucket_handler(Extension(bucket): Extension<BucketDto>) -> Result<JsonResponse> {
//     Ok(JsonResponse::new(serde_json::to_string(&bucket).unwrap()))
// }
//
// pub async fn update_bucket_handler(
//     State(state): State<AppState>,
//     Extension(actor): Extension<Actor>,
//     Extension(bucket): Extension<BucketDto>,
//     payload: CoreResult<Json<UpdateBucket>, JsonRejection>,
// ) -> Result<JsonResponse> {
//     let permissions = vec![Permission::BucketsEdit];
//     ensure!(
//         actor.has_permissions(&permissions),
//         ForbiddenSnafu {
//             msg: "Insufficient permissions"
//         }
//     );
//
//     let data = payload.context(JsonRejectionSnafu {
//         msg: "Invalid request payload",
//     })?;
//
//     let updated = update_bucket(&state, &bucket.id, &data).await?;
//     let updated_bucket = match updated {
//         true => {
//             let mut b = bucket.clone();
//             if let Some(label) = &data.label {
//                 b.label = label.clone();
//             }
//             b
//         }
//         false => bucket,
//     };
//
//     Ok(JsonResponse::new(
//         serde_json::to_string(&updated_bucket).unwrap(),
//     ))
// }
//
// pub async fn delete_bucket_handler(
//     State(state): State<AppState>,
//     Extension(actor): Extension<Actor>,
//     Extension(bucket): Extension<BucketDto>,
// ) -> Result<JsonResponse> {
//     let permissions = vec![Permission::BucketsDelete];
//     ensure!(
//         actor.has_permissions(&permissions),
//         ForbiddenSnafu {
//             msg: "Insufficient permissions"
//         }
//     );
//
//     let _ = delete_bucket(&state, bucket.id.as_str()).await?;
//
//     Ok(JsonResponse::with_status(
//         StatusCode::NO_CONTENT,
//         "".to_string(),
//     ))
// }
//
// pub async fn create_bucket_handler(
//     State(state): State<AppState>,
//     Extension(actor): Extension<Actor>,
//     Extension(client): Extension<ClientDto>,
//     payload: CoreResult<Json<NewBucket>, JsonRejection>,
// ) -> Result<JsonResponse> {
//     let permissions = vec![Permission::BucketsCreate];
//     ensure!(
//         actor.has_permissions(&permissions),
//         ForbiddenSnafu {
//             msg: "Insufficient permissions"
//         }
//     );
//
//     let data = payload.context(JsonRejectionSnafu {
//         msg: "Invalid request payload",
//     })?;
//
//     let bucket = create_bucket(&state, &client.id, &data).await?;
//
//     Ok(JsonResponse::with_status(
//         StatusCode::CREATED,
//         serde_json::to_string(&bucket).unwrap(),
//     ))
// }

pub async fn list_users_handler(
    state: State<AppState>,
    query: Query<ListUsersParamsDto>,
) -> Result<Response<Body>> {
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

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(buffed_result.encode_to_vec()))
        .unwrap())
}

pub async fn create_user_handler(state: State<AppState>, body: Bytes) -> Result<Response<Body>> {
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

    Ok(Response::builder()
        .status(201)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(buffed_user.encode_to_vec()))
        .unwrap())
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

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(buffed_user.encode_to_vec()))
        .unwrap())
}

pub async fn update_user_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    user: Extension<UserDto>,
    body: Bytes,
) -> Result<Response<Body>> {
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

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(buffed_user.encode_to_vec()))
        .unwrap())
}

// pub async fn reset_user_password_handler(
//     State(state): State<AppState>,
//     Extension(actor): Extension<Actor>,
//     Extension(user): Extension<UserDto>,
//     payload: CoreResult<Json<UpdateUserPassword>, JsonRejection>,
// ) -> Result<JsonResponse> {
//     let permissions = vec![Permission::UsersEdit];
//     ensure!(
//         actor.has_permissions(&permissions),
//         ForbiddenSnafu {
//             msg: "Insufficient permissions"
//         }
//     );
//
//     // Do not allow updating your own user
//     ensure!(
//         &actor.user.id != &user.id,
//         ForbiddenSnafu {
//             msg: "Updating your own user account not allowed"
//         }
//     );
//
//     let data = payload.context(JsonRejectionSnafu {
//         msg: "Invalid request payload",
//     })?;
//
//     let _ = update_password(&state, &user.id, &data).await?;
//
//     // Re-query and show
//     let updated_user = state
//         .db
//         .users
//         .get(&user.id)
//         .await
//         .context(DbSnafu)?
//         .context(WhateverSnafu {
//             msg: "Unable to re-query user information.",
//         })?;
//
//     let dto: UserDto = updated_user.into();
//
//     Ok(JsonResponse::new(serde_json::to_string(&dto).unwrap()))
// }
//
pub async fn delete_user_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    user: Extension<UserDto>,
) -> Result<Response<Body>> {
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

    Ok(Response::builder()
        .status(204)
        .header("Content-Type", "application/x-protobuf")
        .body(Body::from(vec![]))
        .unwrap())
}

// pub async fn list_dirs_handler(
//     State(state): State<AppState>,
//     Extension(actor): Extension<Actor>,
//     Extension(bucket): Extension<BucketDto>,
//     query: Query<ListDirsParams>,
// ) -> Result<JsonResponse> {
//     let permissions = vec![Permission::DirsList];
//     ensure!(
//         actor.has_permissions(&permissions),
//         ForbiddenSnafu {
//             msg: "Insufficient permissions"
//         }
//     );
//
//     let dirs = state
//         .db
//         .dirs
//         .list(bucket.id.as_str(), &query)
//         .await
//         .context(DbSnafu)?;
//
//     Ok(JsonResponse::new(serde_json::to_string(&dirs).unwrap()))
// }
//
// pub async fn create_dir_handler(
//     State(state): State<AppState>,
//     Extension(actor): Extension<Actor>,
//     Extension(bucket): Extension<BucketDto>,
//     payload: CoreResult<Json<NewDir>, JsonRejection>,
// ) -> Result<JsonResponse> {
//     let permissions = vec![Permission::DirsCreate];
//     ensure!(
//         actor.has_permissions(&permissions),
//         ForbiddenSnafu {
//             msg: "Insufficient permissions"
//         }
//     );
//
//     let data = payload.context(JsonRejectionSnafu {
//         msg: "Invalid request payload",
//     })?;
//
//     let dir = create_dir(&state, &bucket.id, &data).await?;
//
//     Ok(JsonResponse::with_status(
//         StatusCode::CREATED,
//         serde_json::to_string(&dir).unwrap(),
//     ))
// }
//
// pub async fn get_dir_handler(Extension(dir): Extension<DirDto>) -> Result<JsonResponse> {
//     Ok(JsonResponse::new(serde_json::to_string(&dir).unwrap()))
// }
//
// pub async fn update_dir_handler(
//     State(state): State<AppState>,
//     Extension(actor): Extension<Actor>,
//     Extension(dir): Extension<DirDto>,
//     payload: CoreResult<Json<UpdateDir>, JsonRejection>,
// ) -> Result<JsonResponse> {
//     let permissions = vec![Permission::DirsEdit];
//     ensure!(
//         actor.has_permissions(&permissions),
//         ForbiddenSnafu {
//             msg: "Insufficient permissions"
//         }
//     );
//
//     let data = payload.context(JsonRejectionSnafu {
//         msg: "Invalid request payload",
//     })?;
//
//     let updated = update_dir(&state, &dir.id, &data).await?;
//
//     // Either return the updated dir or the original one
//     match updated {
//         true => get_dir_as_response(&state, &dir.id).await,
//         false => Ok(JsonResponse::new(serde_json::to_string(&dir).unwrap())),
//     }
// }
//
// async fn get_dir_as_response(state: &AppState, id: &str) -> Result<JsonResponse> {
//     let res = state.db.dirs.get(id).await.context(DbSnafu)?;
//     let dir = res.context(WhateverSnafu {
//         msg: "Error getting directory",
//     })?;
//
//     Ok(JsonResponse::new(serde_json::to_string(&dir).unwrap()))
// }
//
// pub async fn delete_dir_handler(
//     State(state): State<AppState>,
//     Extension(actor): Extension<Actor>,
//     Path(params): Path<Params>,
// ) -> Result<JsonResponse> {
//     let permissions = vec![Permission::DirsDelete];
//     ensure!(
//         actor.has_permissions(&permissions),
//         ForbiddenSnafu {
//             msg: "Insufficient permissions"
//         }
//     );
//
//     let dir_id = params.dir_id.clone().expect("dir_id is required");
//     let _ = delete_dir(&state, &dir_id).await?;
//     Ok(JsonResponse::with_status(
//         StatusCode::NO_CONTENT,
//         "".to_string(),
//     ))
// }
//
// pub async fn list_files_handler(
//     State(state): State<AppState>,
//     Extension(actor): Extension<Actor>,
//     Extension(bucket): Extension<BucketDto>,
//     Extension(dir): Extension<DirDto>,
//     query: Query<ListFilesParams>,
// ) -> Result<JsonResponse> {
//     let permissions = vec![Permission::FilesList, Permission::FilesView];
//     ensure!(
//         actor.has_permissions(&permissions),
//         ForbiddenSnafu {
//             msg: "Insufficient permissions"
//         }
//     );
//
//     let files = state.db.files.list(&dir, &query).await.context(DbSnafu)?;
//     let storage_client = state.storage_client.clone();
//
//     // Generate download urls for each files
//     let items = storage_client
//         .format_files(&bucket.name, &dir.name, files.data)
//         .await
//         .context(StorageSnafu)?;
//
//     let listing = Paginated::new(
//         items,
//         files.meta.page,
//         files.meta.per_page,
//         files.meta.total_records,
//     );
//     Ok(JsonResponse::new(serde_json::to_string(&listing).unwrap()))
// }
//
// pub async fn create_file_handler(
//     State(state): State<AppState>,
//     Extension(actor): Extension<Actor>,
//     Extension(bucket): Extension<BucketDto>,
//     Extension(dir): Extension<DirDto>,
//     mut multipart: Multipart,
// ) -> Result<JsonResponse> {
//     let permissions = vec![Permission::FilesCreate];
//     ensure!(
//         actor.has_permissions(&permissions),
//         ForbiddenSnafu {
//             msg: "Insufficient permissions"
//         }
//     );
//
//     let mut payload: Option<FilePayload> = None;
//
//     while let Some(mut field) = multipart.next_field().await.unwrap() {
//         let name = field.name().unwrap().to_string();
//         if name != "file" {
//             continue;
//         }
//
//         let original_filename = field.file_name().unwrap().to_string();
//
//         // Low chance of collision but higher than the full uuid v7 string
//         // Prefer a shorter filename for better readability
//         let filename = slugify_prefixed(&original_filename);
//
//         // Ensure upload dir exists
//         let orig_dir = state
//             .config
//             .upload_dir
//             .clone()
//             .join(ImgVersion::Original.to_string());
//
//         let _ = create_dir_all(orig_dir.clone())
//             .await
//             .context(UploadDirSnafu)?;
//
//         // Prepare to save to file
//         let file_path = orig_dir.as_path().join(&filename);
//         let mut file = File::create(&file_path)
//             .await
//             .context(CreateFileSnafu { path: file_path })?;
//
//         // Stream contents to file
//         let mut size: usize = 0;
//         while let Some(chunk) = field.chunk().await.unwrap() {
//             size += chunk.len();
//             file.write_all(&chunk).await.unwrap();
//         }
//
//         payload = Some({
//             FilePayload {
//                 upload_dir: state.config.upload_dir.clone(),
//                 name: original_filename,
//                 filename: filename.clone(),
//                 path: orig_dir.clone().join(&filename),
//                 size: size as i64,
//             }
//         })
//     }
//
//     let payload = payload.context(MissingUploadFileSnafu {
//         msg: "Missing upload file",
//     })?;
//
//     let storage_client = state.storage_client.clone();
//     let file = create_file(state, &bucket, &dir, &payload).await?;
//     let file_dto: FileDto = file.into();
//     let file_dto = storage_client
//         .format_file(&bucket.name, &dir.name, file_dto)
//         .await
//         .context(StorageSnafu)?;
//
//     Ok(JsonResponse::with_status(
//         StatusCode::CREATED,
//         serde_json::to_string(&file_dto).unwrap(),
//     ))
// }
//
// pub async fn get_file_handler(
//     State(state): State<AppState>,
//     Extension(bucket): Extension<BucketDto>,
//     Extension(dir): Extension<DirDto>,
//     Extension(file): Extension<FileObject>,
// ) -> Result<JsonResponse> {
//     let storage_client = state.storage_client.clone();
//     // Extract dir from the middleware extension
//     let file_dto: FileDto = file.clone().into();
//     let file_dto = storage_client
//         .format_file(&bucket.name, &dir.name, file_dto)
//         .await
//         .context(StorageSnafu)?;
//     Ok(JsonResponse::new(serde_json::to_string(&file_dto).unwrap()))
// }
//
// pub async fn delete_file_handler(
//     State(state): State<AppState>,
//     Extension(actor): Extension<Actor>,
//     Extension(bucket): Extension<BucketDto>,
//     Extension(dir): Extension<DirDto>,
//     Extension(file): Extension<FileObject>,
// ) -> Result<JsonResponse> {
//     let permissions = vec![Permission::FilesDelete];
//     ensure!(
//         actor.has_permissions(&permissions),
//         ForbiddenSnafu {
//             msg: "Insufficient permissions"
//         }
//     );
//
//     // Delete record
//     let _ = state.db.files.delete(&file.id).await.context(DbSnafu)?;
//
//     // Delete file(s) from storage
//     let storage_client = state.storage_client.clone();
//     let dto: FileDto = file.into();
//     let _ = storage_client
//         .delete_file_object(&bucket.name, &dir.name, &dto)
//         .await
//         .context(StorageSnafu)?;
//
//     Ok(JsonResponse::with_status(
//         StatusCode::NO_CONTENT,
//         "".to_string(),
//     ))
// }
