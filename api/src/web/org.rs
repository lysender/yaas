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
        dto::{
            NewOrgBuf, OrgAppSuggestionBuf, OrgBuf, OrgMemberSuggestionBuf, OrgOwnerSuggestionBuf,
            PaginatedOrgAppSuggestionsBuf, PaginatedOrgMemberSuggestionsBuf,
            PaginatedOrgOwnerSuggestionsBuf, PaginatedOrgsBuf, UpdateOrgBuf,
        },
        pagination::PaginatedMetaBuf,
    },
    dto::{
        Actor, ListOrgAppsParamsDto, ListOrgMembersParamsDto, ListOrgOwnerSuggestionsParamsDto,
        ListOrgsParamsDto, NewOrgDto, OrgDto, UpdateOrgDto,
    },
    role::Permission,
    validators::flatten_errors,
};

use crate::{
    Error, Result,
    error::{ForbiddenSnafu, ValidationSnafu, WhateverSnafu},
    services::{
        org::{
            create_org_svc, delete_org_svc, get_org_svc, list_org_owner_suggestions_svc,
            list_orgs_svc, update_org_svc,
        },
        org_app::list_org_app_suggestions_svc,
        org_member::list_org_member_suggestions_svc,
    },
    state::AppState,
    web::{build_response, middleware::org_middleware, org_apps_routes, org_members_routes},
};

pub fn orgs_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(list_orgs_handler).post(create_org_handler))
        .route(
            "/owner-suggestions",
            get(list_org_owner_suggestions_handler),
        )
        .nest("/{org_id}", inner_org_routes(state.clone()))
        .with_state(state)
}

fn inner_org_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route(
            "/",
            get(get_org_handler)
                .patch(update_org_handler)
                .delete(delete_org_handler),
        )
        .route(
            "/member-suggestions",
            get(list_org_member_suggestions_handler),
        )
        .route("/app-suggestions", get(list_org_app_suggestions_handler))
        .nest("/members", org_members_routes(state.clone()))
        .nest("/apps", org_apps_routes(state.clone()))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            org_middleware,
        ))
        .with_state(state)
}

async fn list_orgs_handler(
    Extension(actor): Extension<Actor>,
    State(state): State<AppState>,
    Query(query): Query<ListOrgsParamsDto>,
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

    // Only superusers can list all orgs
    // Other users can only list their own org
    if !actor.is_system_admin() {
        let actor = actor.actor.as_ref().expect("Actor should be present");
        let org_id = actor.org_id;

        let org = get_org_svc(&state, org_id).await?;
        let org = org.context(WhateverSnafu {
            msg: "Unable to find org information.",
        })?;

        let buffed_meta = PaginatedMetaBuf {
            page: 1,
            per_page: 1,
            total_records: 1,
            total_pages: 1,
        };

        let buffed_list: Vec<OrgBuf> = vec![OrgBuf {
            id: org.id,
            name: org.name,
            status: org.status,
            owner_id: org.owner_id,
            owner_email: org.owner_email,
            owner_name: org.owner_name,
            created_at: org.created_at,
            updated_at: org.updated_at,
        }];

        return Ok(build_response(
            200,
            PaginatedOrgsBuf {
                meta: Some(buffed_meta),
                data: buffed_list,
            }
            .encode_to_vec(),
        ));
    }

    let orgs = list_orgs_svc(&state, query).await?;
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

async fn list_org_owner_suggestions_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Query(query): Query<ListOrgOwnerSuggestionsParamsDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::UsersList, Permission::OrgsCreate];
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

    let suggestions = list_org_owner_suggestions_svc(&state, query).await?;
    let buffed_meta = PaginatedMetaBuf {
        page: suggestions.meta.page,
        per_page: suggestions.meta.per_page,
        total_records: suggestions.meta.total_records,
        total_pages: suggestions.meta.total_pages,
    };
    let buffed_list: Vec<OrgOwnerSuggestionBuf> = suggestions
        .data
        .into_iter()
        .map(|suggestion| OrgOwnerSuggestionBuf {
            id: suggestion.id,
            email: suggestion.email,
            name: suggestion.name,
        })
        .collect();

    let buffed_result = PaginatedOrgOwnerSuggestionsBuf {
        meta: Some(buffed_meta),
        data: buffed_list,
    };

    Ok(build_response(200, buffed_result.encode_to_vec()))
}

async fn create_org_handler(
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

async fn get_org_handler(Extension(org): Extension<OrgDto>) -> Result<Response<Body>> {
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

async fn update_org_handler(
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

async fn delete_org_handler(
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

async fn list_org_member_suggestions_handler(
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

    let buffed_result = PaginatedOrgMemberSuggestionsBuf {
        meta: Some(buffed_meta),
        data: buffed_list,
    };

    Ok(build_response(200, buffed_result.encode_to_vec()))
}

async fn list_org_app_suggestions_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Extension(org): Extension<OrgDto>,
    Query(query): Query<ListOrgAppsParamsDto>,
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

    let suggestions = list_org_app_suggestions_svc(&state, org.id, query).await?;
    let buffed_meta = PaginatedMetaBuf {
        page: suggestions.meta.page,
        per_page: suggestions.meta.per_page,
        total_records: suggestions.meta.total_records,
        total_pages: suggestions.meta.total_pages,
    };
    let buffed_list: Vec<OrgAppSuggestionBuf> = suggestions
        .data
        .into_iter()
        .map(|suggestion| OrgAppSuggestionBuf {
            id: suggestion.id,
            name: suggestion.name,
        })
        .collect();

    let buffed_result = PaginatedOrgAppSuggestionsBuf {
        meta: Some(buffed_meta),
        data: buffed_list,
    };

    Ok(build_response(200, buffed_result.encode_to_vec()))
}
