use axum::{
    Extension, Router,
    body::Body,
    extract::{Query, State},
    http::StatusCode,
    middleware,
    response::Response,
    routing::get,
};
use snafu::{OptionExt, ensure};
use validator::Validate;

use yaas::{
    dto::{
        Actor, ListOrgAppsParamsDto, ListOrgMembersParamsDto, ListOrgOwnerSuggestionsParamsDto,
        ListOrgsParamsDto, NewOrgDto, OrgDto, UpdateOrgDto,
    },
    role::Permission,
    validators::flatten_errors,
};

use crate::{
    Result,
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
    web::{
        empty_response,
        json_input::{JsonPayload, parse_and_validate_json},
        json_response,
        middleware::org_middleware,
        org_apps_routes, org_members_routes,
    },
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
        let org_id = actor.org_id.clone();

        let org = get_org_svc(&state, &org_id).await?;
        let org = org.context(WhateverSnafu {
            msg: "Unable to find org information.",
        })?;

        return Ok(json_response(
            StatusCode::OK,
            yaas::pagination::Paginated {
                meta: yaas::pagination::PaginatedMeta {
                    page: 1,
                    per_page: 50,
                    total_records: 1,
                    total_pages: 1,
                },
                data: vec![org],
            },
        ));
    }

    let orgs = list_orgs_svc(&state, query).await?;
    Ok(json_response(StatusCode::OK, orgs))
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
    Ok(json_response(StatusCode::OK, suggestions))
}

async fn create_org_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    payload: JsonPayload<NewOrgDto>,
) -> Result<Response<Body>> {
    let permissions = vec![Permission::OrgsCreate];
    ensure!(
        actor.has_permissions(&permissions),
        ForbiddenSnafu {
            msg: "Insufficient permissions"
        }
    );

    let data = parse_and_validate_json(payload)?;

    let org = create_org_svc(&state, data).await?;
    Ok(json_response(StatusCode::CREATED, org))
}

async fn get_org_handler(Extension(org): Extension<OrgDto>) -> Result<Response<Body>> {
    Ok(json_response(StatusCode::OK, org))
}

async fn update_org_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    org: Extension<OrgDto>,
    payload: JsonPayload<UpdateOrgDto>,
) -> Result<Response<Body>> {
    let org_id = org.id.clone();

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
            actor.org_id != org_id,
            ForbiddenSnafu {
                msg: "Superusers cannot update their own organization"
            }
        );
    }

    let data = parse_and_validate_json(payload)?;

    let _ = update_org_svc(&state, &org_id, data).await?;

    // Not ideal but we need to re-query to get the updated data
    let updated_org = get_org_svc(&state, &org_id).await?;
    let updated_org = updated_org.context(WhateverSnafu {
        msg: "Unable to re-query org information.",
    })?;

    Ok(json_response(StatusCode::OK, updated_org))
}

async fn delete_org_handler(
    state: State<AppState>,
    actor: Extension<Actor>,
    org: Extension<OrgDto>,
) -> Result<Response<Body>> {
    let org = org.0;
    let org_id = org.id.clone();

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

    let _ = delete_org_svc(&state, &org_id).await?;

    Ok(empty_response(StatusCode::NO_CONTENT))
}

async fn list_org_member_suggestions_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Extension(org): Extension<OrgDto>,
    Query(query): Query<ListOrgMembersParamsDto>,
) -> Result<Response<Body>> {
    let org_id = org.id.clone();

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

    let members = list_org_member_suggestions_svc(&state, &org_id, query).await?;
    Ok(json_response(StatusCode::OK, members))
}

async fn list_org_app_suggestions_handler(
    State(state): State<AppState>,
    Extension(actor): Extension<Actor>,
    Extension(org): Extension<OrgDto>,
    Query(query): Query<ListOrgAppsParamsDto>,
) -> Result<Response<Body>> {
    let org_id = org.id.clone();

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

    let suggestions = list_org_app_suggestions_svc(&state, &org_id, query).await?;
    Ok(json_response(StatusCode::OK, suggestions))
}
