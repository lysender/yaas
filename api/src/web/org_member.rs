use axum::{
    Extension, Router,
    body::{Body, Bytes},
    extract::{Query, State},
    middleware,
    response::Response,
    routing::get,
};
use prost::Message;
use snafu::{OptionExt, ensure};
use validator::Validate;

use yaas::{
    buffed::{
        dto::{NewOrgMemberBuf, OrgMemberBuf, PaginatedOrgMembersBuf, UpdateOrgMemberBuf},
        pagination::PaginatedMetaBuf,
    },
    dto::{
        Actor, ListOrgMembersParamsDto, NewOrgMemberDto, OrgDto, OrgMemberDto, UpdateOrgMemberDto,
    },
    role::{Permission, to_buffed_roles},
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
    web::{build_response, middleware::org_member_middleware},
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

async fn get_org_member_handler(member: Extension<OrgMemberDto>) -> Result<Response<Body>> {
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
