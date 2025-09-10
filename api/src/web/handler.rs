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
            ChangeCurrentPasswordBuf, ErrorMessageBuf, NewUserBuf, NewUserWithPasswordBuf,
            OrgMembershipBuf, PaginatedUsersBuf, SetupBodyBuf, SuperuserBuf, UserBuf,
        },
        pagination::PaginatedMetaBuf,
    },
    dto::{
        ChangeCurrentPasswordDto, ErrorMessageDto, ListUsersParamsDto, NewUserDto,
        NewUserWithPasswordDto, SetupBodyDto, UserDto,
    },
    role::{to_buffed_permissions, to_buffed_roles},
    validators::flatten_errors,
};

use crate::{
    Error, Result,
    auth::authenticate,
    error::ValidationSnafu,
    health::{check_liveness, check_readiness},
    services::{
        password::change_current_password_svc,
        superuser::setup_superuser_svc,
        user::{create_user_svc, delete_user_svc, list_users_svc},
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

// pub async fn list_orgs_handler(
//     state: State<AppState>,
//     actor: Extension<Actor>,
// ) -> Result<JsonResponse> {
//     let clients = state.db.orgs.list(client_id).await.context(DbSnafu)?;
//     Ok(JsonResponse::new(serde_json::to_string(&clients).unwrap()))
// }

// pub async fn create_client_handler(
//     State(state): State<AppState>,
//     Extension(actor): Extension<Actor>,
//     payload: CoreResult<Json<NewClient>, JsonRejection>,
// ) -> Result<JsonResponse> {
//     let permissions = vec![Permission::ClientsCreate];
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
//     let created = create_client(&state, &data, false).await?;
//     let dto: ClientDto = created.into();
//     Ok(JsonResponse::new(serde_json::to_string(&dto).unwrap()))
// }
//
// pub async fn get_client_handler(Extension(client): Extension<ClientDto>) -> Result<JsonResponse> {
//     Ok(JsonResponse::new(serde_json::to_string(&client).unwrap()))
// }
//
// pub async fn update_client_handler(
//     State(state): State<AppState>,
//     Extension(actor): Extension<Actor>,
//     Extension(client): Extension<ClientDto>,
//     payload: CoreResult<Json<UpdateClient>, JsonRejection>,
// ) -> Result<JsonResponse> {
//     let permissions = vec![Permission::ClientsEdit];
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
//     // No changes, just return the client
//     if data.name.is_none() && data.default_bucket_id.is_none() && data.status.is_none() {
//         return Ok(JsonResponse::new(serde_json::to_string(&client).unwrap()));
//     }
//
//     let updated = update_client(&state, client.id.as_str(), &data).await?;
//     if !updated {
//         // No changes, just return the client
//         return Ok(JsonResponse::new(serde_json::to_string(&client).unwrap()));
//     }
//
//     let updated_client = state
//         .db
//         .clients
//         .get(client.id.as_str())
//         .await
//         .context(DbSnafu)?;
//
//     let updated_client = updated_client.context(WhateverSnafu {
//         msg: "Unable to find updated client",
//     })?;
//
//     Ok(JsonResponse::new(
//         serde_json::to_string(&updated_client).unwrap(),
//     ))
// }
//
// pub async fn delete_client_handler(
//     State(state): State<AppState>,
//     Extension(actor): Extension<Actor>,
//     Extension(client): Extension<ClientDto>,
// ) -> Result<JsonResponse> {
//     let permissions = vec![Permission::ClientsDelete];
//     ensure!(
//         actor.has_permissions(&permissions),
//         ForbiddenSnafu {
//             msg: "Insufficient permissions"
//         }
//     );
//     ensure!(
//         !client.admin,
//         ForbiddenSnafu {
//             msg: "Cannot delete admin client"
//         }
//     );
//
//     let _ = delete_client(&state, &client.id).await?;
//
//     Ok(JsonResponse::with_status(
//         StatusCode::NO_CONTENT,
//         "".to_string(),
//     ))
// }
//
// pub async fn update_default_bucket_handler(
//     State(state): State<AppState>,
//     Extension(actor): Extension<Actor>,
//     Extension(client): Extension<ClientDto>,
//     payload: CoreResult<Json<ClientDefaultBucket>, JsonRejection>,
// ) -> Result<JsonResponse> {
//     let permissions = vec![Permission::ClientsEdit];
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
//     let data = UpdateClient {
//         name: None,
//         status: None,
//         default_bucket_id: Some(data.default_bucket_id.clone()),
//     };
//
//     let updated = update_client(&state, &client.id, &data).await?;
//     if !updated {
//         // No changes, just return the client
//         return Ok(JsonResponse::new(serde_json::to_string(&client).unwrap()));
//     }
//
//     let updated_client = state.db.clients.get(&client.id).await.context(DbSnafu)?;
//     let updated_client = updated_client.context(WhateverSnafu {
//         msg: "Unable to find updated client",
//     })?;
//
//     Ok(JsonResponse::new(
//         serde_json::to_string(&updated_client).unwrap(),
//     ))
// }
//
// pub async fn list_buckets_handler(
//     State(state): State<AppState>,
//     Extension(actor): Extension<Actor>,
//     Extension(client): Extension<ClientDto>,
// ) -> Result<JsonResponse> {
//     let permissions = vec![Permission::BucketsList];
//     ensure!(
//         actor.has_permissions(&permissions),
//         ForbiddenSnafu {
//             msg: "Insufficient permissions"
//         }
//     );
//     let buckets = state.db.buckets.list(&client.id).await.context(DbSnafu)?;
//     Ok(JsonResponse::new(serde_json::to_string(&buckets).unwrap()))
// }
//
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

pub async fn create_user_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    body: Bytes,
) -> Result<Response<Body>> {
    // Parse body as protobuf message
    let Ok(payload) = NewUserWithPasswordBuf::decode(body) else {
        return Err(Error::BadProtobuf);
    };

    let data: NewUserWithPasswordDto = payload.into();
    let errors = data.validate();
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

pub async fn get_user_handler(
    actor: Extension<Actor>,
    user: Extension<UserDto>,
) -> Result<Response<Body>> {
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

// pub async fn update_user_status_handler(
//     State(state): State<AppState>,
//     Extension(actor): Extension<Actor>,
//     Extension(user): Extension<UserDto>,
//     payload: CoreResult<Json<UpdateUserStatus>, JsonRejection>,
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
//     // Ideally, should not update if status do not change
//     let _ = update_user_status(&state, &user.id, &data).await?;
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
