use axum::{
    Extension,
    body::Body,
    extract::{Path, Request, State},
    http::header,
    middleware::Next,
    response::Response,
};
use snafu::{OptionExt, ResultExt, ensure};
use yaas::{actor::Actor, role::Permission, utils::valid_id};

use crate::{
    Result,
    auth::authenticate_token,
    error::{
        BadRequestSnafu, DbSnafu, ForbiddenSnafu, InsufficientAuthScopeSnafu,
        InvalidAuthTokenSnafu, NotFoundSnafu,
    },
    state::AppState,
    web::params::{AppParams, OrgAppParams, OrgMemberParams, OrgParams, Params, UserParams},
};

pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response<Body>> {
    // Middleware to extract actor information from the request
    // Do not enforce authentication here, just extract the actor information
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    // Start with an empty actor
    let mut actor: Actor = Actor::default();

    if let Some(auth_header) = auth_header {
        // At this point, authentication must be verified
        ensure!(auth_header.starts_with("Bearer "), InvalidAuthTokenSnafu);
        let token = auth_header.replace("Bearer ", "");

        actor = authenticate_token(&state, &token).await?;
    }

    // Forward to the next middleware/handler passing the actor information
    request.extensions_mut().insert(actor);

    let response = next.run(request).await;
    Ok(response)
}

pub async fn require_auth_middleware(
    actor: Extension<Actor>,
    request: Request,
    next: Next,
) -> Result<Response<Body>> {
    ensure!(actor.has_auth_scope(), InsufficientAuthScopeSnafu);

    Ok(next.run(request).await)
}

pub async fn user_middleware(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Path(params): Path<UserParams>,
    mut request: Request,
    next: Next,
) -> Result<Response<Body>> {
    let doc = state.db.users.get(params.user_id).await.context(DbSnafu)?;
    let doc = doc.context(NotFoundSnafu {
        msg: "User not found",
    })?;

    // Forward to the next middleware/handler passing the user information
    request.extensions_mut().insert(doc);
    let response = next.run(request).await;
    Ok(response)
}

pub async fn app_middleware(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Path(params): Path<AppParams>,
    mut request: Request,
    next: Next,
) -> Result<Response<Body>> {
    let doc = state.db.apps.get(params.app_id).await.context(DbSnafu)?;
    let doc = doc.context(NotFoundSnafu {
        msg: "App not found",
    })?;

    // Forward to the next middleware/handler passing the app information
    request.extensions_mut().insert(doc);
    let response = next.run(request).await;
    Ok(response)
}

pub async fn org_middleware(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Path(params): Path<OrgParams>,
    mut request: Request,
    next: Next,
) -> Result<Response<Body>> {
    let doc = state.db.orgs.get(params.org_id).await.context(DbSnafu)?;
    let doc = doc.context(NotFoundSnafu {
        msg: "Org not found",
    })?;

    // Forward to the next middleware/handler passing the org information
    request.extensions_mut().insert(doc);
    let response = next.run(request).await;
    Ok(response)
}

pub async fn org_member_middleware(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Path(params): Path<OrgMemberParams>,
    mut request: Request,
    next: Next,
) -> Result<Response<Body>> {
    let doc = state
        .db
        .org_members
        .get(params.org_member_id)
        .await
        .context(DbSnafu)?;

    let doc = doc.context(NotFoundSnafu {
        msg: "Org member not found",
    })?;

    // Forward to the next middleware/handler passing the org member information
    request.extensions_mut().insert(doc);
    let response = next.run(request).await;
    Ok(response)
}

pub async fn org_app_middleware(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Path(params): Path<OrgAppParams>,
    mut request: Request,
    next: Next,
) -> Result<Response<Body>> {
    let doc = state
        .db
        .org_apps
        .get(params.org_app_id)
        .await
        .context(DbSnafu)?;

    let doc = doc.context(NotFoundSnafu {
        msg: "Org app not found",
    })?;

    // Forward to the next middleware/handler passing the org member information
    request.extensions_mut().insert(doc);
    let response = next.run(request).await;
    Ok(response)
}

// pub async fn client_middleware(
//     State(state): State<AppState>,
//     Extension(actor): Extension<Actor>,
//     Path(params): Path<ClientParams>,
//     mut request: Request,
//     next: Next,
// ) -> Result<Response<Body>> {
//     let permissions = vec![Permission::ClientsView];
//     ensure!(
//         actor.has_permissions(&permissions),
//         ForbiddenSnafu {
//             msg: "Insufficient permissions"
//         }
//     );
//
//     ensure!(
//         valid_id(&params.client_id),
//         BadRequestSnafu {
//             msg: "Invalid client id"
//         }
//     );
//
//     // Ensure regular clients can only view their own clients
//     if !actor.is_system_admin() {
//         ensure!(
//             actor.client_id.as_str() == params.client_id.as_str(),
//             NotFoundSnafu {
//                 msg: "Client not found"
//             }
//         )
//     }
//
//     let client = state
//         .db
//         .clients
//         .get(&params.client_id)
//         .await
//         .context(DbSnafu)?;
//
//     let client = client.context(NotFoundSnafu {
//         msg: "Client not found",
//     })?;
//
//     // Forward to the next middleware/handler passing the client information
//     request.extensions_mut().insert(client);
//     let response = next.run(request).await;
//     Ok(response)
// }
//
// pub async fn bucket_middleware(
//     State(state): State<AppState>,
//     Extension(actor): Extension<Actor>,
//     Path(params): Path<Params>,
//     mut request: Request,
//     next: Next,
// ) -> Result<Response<Body>> {
//     ensure!(
//         actor.has_files_scope(),
//         ForbiddenSnafu {
//             msg: "Insufficient auth scope"
//         }
//     );
//
//     let permissions = vec![Permission::BucketsList, Permission::BucketsView];
//     ensure!(
//         actor.has_permissions(&permissions),
//         ForbiddenSnafu {
//             msg: "Insufficient permissions"
//         }
//     );
//
//     ensure!(
//         valid_id(&params.bucket_id),
//         BadRequestSnafu {
//             msg: "Invalid bucket id"
//         }
//     );
//
//     let bucket = state
//         .db
//         .buckets
//         .get(&params.bucket_id)
//         .await
//         .context(DbSnafu)?;
//
//     let bucket = bucket.context(NotFoundSnafu {
//         msg: "Bucket not found",
//     })?;
//
//     if !actor.is_system_admin() {
//         ensure!(
//             &bucket.client_id == &actor.client_id,
//             NotFoundSnafu {
//                 msg: "Bucket not found"
//             }
//         );
//     }
//
//     // Forward to the next middleware/handler passing the bucket information
//     request.extensions_mut().insert(bucket);
//     let response = next.run(request).await;
//     Ok(response)
// }
//
// pub async fn user_middleware(
//     State(state): State<AppState>,
//     Extension(actor): Extension<Actor>,
//     Path(params): Path<UserParams>,
//     mut request: Request,
//     next: Next,
// ) -> Result<Response<Body>> {
//     let permissions = vec![Permission::UsersList, Permission::UsersView];
//     ensure!(
//         actor.has_permissions(&permissions),
//         ForbiddenSnafu {
//             msg: "Insufficient permissions"
//         }
//     );
//
//     ensure!(
//         valid_id(&params.user_id),
//         BadRequestSnafu {
//             msg: "Invalid user id"
//         }
//     );
//
//     let user = state.db.users.get(&params.user_id).await.context(DbSnafu)?;
//     let user = user.context(NotFoundSnafu {
//         msg: "User not found",
//     })?;
//
//     if !actor.is_system_admin() {
//         ensure!(
//             &user.client_id == &actor.client_id,
//             NotFoundSnafu {
//                 msg: "User not found"
//             }
//         );
//     }
//
//     let user: UserDto = user.into();
//
//     // Forward to the next middleware/handler passing the bucket information
//     request.extensions_mut().insert(user);
//     let response = next.run(request).await;
//     Ok(response)
// }
//
// pub async fn dir_middleware(
//     state: State<AppState>,
//     Extension(actor): Extension<Actor>,
//     Path(params): Path<Params>,
//     mut request: Request,
//     next: Next,
// ) -> Result<Response<Body>> {
//     ensure!(
//         actor.has_files_scope(),
//         ForbiddenSnafu {
//             msg: "Insufficient auth scope"
//         }
//     );
//
//     let permissions = vec![Permission::DirsList, Permission::DirsView];
//     ensure!(
//         actor.has_permissions(&permissions),
//         ForbiddenSnafu {
//             msg: "Insufficient permissions"
//         }
//     );
//
//     let did = params.dir_id.clone().expect("dir_id is required");
//     let dir_res = state.db.dirs.get(&did).await.context(DbSnafu)?;
//
//     let dir = dir_res.context(NotFoundSnafu {
//         msg: "Directory not found",
//     })?;
//
//     let dto: DirDto = dir.into();
//
//     ensure!(
//         &dto.bucket_id == &params.bucket_id,
//         NotFoundSnafu {
//             msg: "Directory not found"
//         }
//     );
//
//     // Forward to the next middleware/handler passing the directory information
//     request.extensions_mut().insert(dto);
//     let response = next.run(request).await;
//     Ok(response)
// }
//
// pub async fn file_middleware(
//     state: State<AppState>,
//     Extension(actor): Extension<Actor>,
//     Path(params): Path<Params>,
//     mut request: Request,
//     next: Next,
// ) -> Result<Response<Body>> {
//     let permissions = vec![Permission::FilesList, Permission::FilesView];
//     ensure!(
//         actor.has_permissions(&permissions),
//         ForbiddenSnafu {
//             msg: "Insufficient permissions"
//         }
//     );
//
//     let did = params.dir_id.clone().expect("dir_id is required");
//     let fid = params.file_id.clone().expect("file_id is required");
//     let file_res = state.db.files.get(&fid).await.context(DbSnafu)?;
//     let file = file_res.context(NotFoundSnafu {
//         msg: "File not found",
//     })?;
//
//     ensure!(
//         &file.dir_id == &did,
//         NotFoundSnafu {
//             msg: "File not found"
//         }
//     );
//
//     // Forward to the next middleware/handler passing the file information
//     request.extensions_mut().insert(file);
//     let response = next.run(request).await;
//     Ok(response)
// }
