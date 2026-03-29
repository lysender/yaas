use axum::{
    Extension, Router,
    body::{Body, Bytes},
    extract::{Query, State},
    http::StatusCode,
    middleware,
    response::Response,
    routing::get,
};
use prost::Message;
use snafu::{OptionExt, ensure};
use validator::Validate;

use yaas::{
    buffed::dto::{NewOrgMemberBuf, UpdateOrgMemberBuf},
    dto::{
        Actor, ListOrgMembersParamsDto, NewOrgMemberDto, OrgDto, OrgMemberDto, UpdateOrgMemberDto,
    },
    role::Permission,
    validators::flatten_errors,
};

use crate::{
    Error, Result,
    error::{ForbiddenSnafu, ValidationSnafu, WhateverSnafu},
    services::org_member::{
        create_org_member_svc, delete_org_member_svc, get_org_member_svc, list_org_members_svc,
        update_org_member_svc,
    },
    state::AppState,
    web::{empty_response, json_response, middleware::org_member_middleware},
};

pub fn org_members_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(list_org_members_handler).post(create_org_member_handler),
        )
        .nest("/{user_id}", org_members_inner_routes(state.clone()))
        .with_state(state)
}

fn org_members_inner_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(get_org_member_handler)
                .patch(update_org_member_handler)
                .delete(delete_org_member_handler),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            org_member_middleware,
        ))
        .with_state(state)
}

async fn list_org_members_handler(
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

    let org_id = org.id.clone();
    let members = list_org_members_svc(&state, &org_id, query.0).await?;
    Ok(json_response(StatusCode::OK, members))
}

async fn create_org_member_handler(
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

    let org_id = org.id.clone();
    let member = create_org_member_svc(&state, &org_id, data).await?;

    // Not ideal but we need to re-query to get the full member details
    let member = get_org_member_svc(&state, &org_id, &member.user_id).await?;
    let member = member.context(WhateverSnafu {
        msg: "Unable to re-query org member information.",
    })?;

    Ok(json_response(StatusCode::CREATED, member))
}

async fn get_org_member_handler(member: Extension<OrgMemberDto>) -> Result<Response<Body>> {
    Ok(json_response(StatusCode::OK, member.0))
}

async fn update_org_member_handler(
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

    let member_id = member.id.clone();
    let member_org_id = member.org_id.clone();
    let member_user_id = member.user_id.clone();

    if actor.org_id == member_org_id {
        ensure!(
            actor.user.id != member_user_id,
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

    let _ = update_org_member_svc(&state, &member_id, data).await?;

    // Not ideal but we need to re-query to get the updated data
    let updated_member = get_org_member_svc(&state, &member_org_id, &member_user_id).await?;
    let updated_member = updated_member.context(WhateverSnafu {
        msg: "Unable to re-query org member information.",
    })?;

    Ok(json_response(StatusCode::OK, updated_member))
}

async fn delete_org_member_handler(
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

    let member_id = member.id.clone();
    let member_org_id = member.org_id.clone();
    let member_user_id = member.user_id.clone();

    if actor.org_id == member_org_id {
        ensure!(
            actor.user.id != member_user_id,
            ForbiddenSnafu {
                msg: "Deleting yourself from the organization is not allowed"
            }
        );
    }

    delete_org_member_svc(&state, &member_id).await?;

    Ok(empty_response(StatusCode::NO_CONTENT))
}
