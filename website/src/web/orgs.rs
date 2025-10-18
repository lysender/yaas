use askama::Template;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::{Extension, Form, body::Body, extract::State, response::Response};
use axum::{Router, middleware, routing::get};
use snafu::{ResultExt, ensure};
use urlencoding::encode;
use validator::Validate;

use crate::error::ValidationSnafu;
use crate::models::{PaginationLinks, UserParams};
use crate::services::users::get_user_svc;
use crate::services::{
    UpdateOrgFormData, UpdateOrgOwnerFormData, create_org_svc, get_org_member_svc,
    list_org_members_svc, list_org_owner_suggestions_svc, list_orgs_svc, update_org_owner_svc,
    update_org_svc,
};
use crate::web::middleware::org_middleware;
use crate::web::{org_apps_routes, org_members_routes};
use crate::{
    Error, Result,
    ctx::Ctx,
    error::{ErrorInfo, ResponseBuilderSnafu, TemplateSnafu},
    models::{Pref, TemplateData},
    run::AppState,
    services::{NewOrgFormData, token::create_csrf_token_svc},
    web::{Action, Resource, enforce_policy},
};
use yaas::dto::OrgDto;
use yaas::dto::{
    ListOrgMembersParamsDto, ListOrgOwnerSuggestionsParamsDto, ListOrgsParamsDto, OrgMemberDto,
    OrgOwnerSuggestionDto,
};
use yaas::role::Permission;
use yaas::validators::flatten_errors;

pub fn orgs_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(orgs_handler))
        .route("/search", get(search_orgs_handler))
        .route("/search-owner", get(search_org_owner_handler))
        .route("/select-owner/{user_id}", get(select_org_owner_handler))
        .route("/new", get(new_org_handler).post(post_new_org_handler))
        .nest("/{org_id}", org_inner_routes(state.clone()))
        .with_state(state)
}

fn org_inner_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(org_page_handler))
        .route("/edit-controls", get(org_controls_handler))
        .route("/edit", get(edit_org_handler).post(post_edit_org_handler))
        .route(
            "/change-owner",
            get(change_org_owner_handler).post(post_change_org_owner_handler),
        )
        .route("/search-owner", get(search_new_org_owner_handler))
        .route("/select-owner/{user_id}", get(select_new_org_owner_handler))
        // .route(
        //     "/delete",
        //     get(delete_user_handler).post(post_delete_user_handler),
        // )
        .nest("/members", org_members_routes(state.clone()))
        .nest("/apps", org_apps_routes(state.clone()))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            org_middleware,
        ))
        .with_state(state)
}

#[derive(Template)]
#[template(path = "pages/orgs/index.html")]
struct OrgsPageTemplate {
    t: TemplateData,
    query_params: String,
}

async fn orgs_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    State(state): State<AppState>,
    Query(query): Query<ListOrgsParamsDto>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::Org, Action::Read)?;

    let errors = query.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);
    t.title = String::from("Orgs");

    let tpl = OrgsPageTemplate {
        t,
        query_params: query.to_string(),
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/orgs/search.html")]
struct SearchOrgsTemplate {
    orgs: Vec<OrgDto>,
    pagination: Option<PaginationLinks>,
    error_message: Option<String>,
}
async fn search_orgs_handler(
    Extension(ctx): Extension<Ctx>,
    State(state): State<AppState>,
    Query(query): Query<ListOrgsParamsDto>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::Org, Action::Read)?;

    let mut tpl = SearchOrgsTemplate {
        orgs: Vec::new(),
        pagination: None,
        error_message: None,
    };

    let keyword = query.keyword.clone();

    match list_orgs_svc(&state, &ctx, query).await {
        Ok(orgs) => {
            let mut keyword_param: String = "".to_string();
            if let Some(keyword) = &keyword {
                keyword_param = format!("&keyword={}", encode(keyword).to_string());
            }
            tpl.orgs = orgs.data;
            tpl.pagination = Some(PaginationLinks::new(
                &orgs.meta,
                "/orgs/search",
                "/orgs",
                &keyword_param,
                ".album-items",
            ));

            Ok(Response::builder()
                .status(200)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
        Err(err) => {
            let error_info = ErrorInfo::from(&err);
            tpl.error_message = Some(error_info.message);

            Ok(Response::builder()
                .status(error_info.status_code)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
    }
}

#[derive(Template)]
#[template(path = "widgets/orgs/search_owner.html")]
struct SearchOwnerTemplate {
    users: Vec<OrgOwnerSuggestionDto>,
    error_message: Option<String>,
}

async fn search_org_owner_handler(
    Extension(ctx): Extension<Ctx>,
    State(state): State<AppState>,
    Query(query): Query<ListOrgOwnerSuggestionsParamsDto>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Read)?;

    let mut tpl = SearchOwnerTemplate {
        users: Vec::new(),
        error_message: None,
    };

    let mut updated_query = query.clone();
    updated_query.per_page = Some(10);

    match list_org_owner_suggestions_svc(&state, &ctx, updated_query).await {
        Ok(users) => {
            tpl.users = users.data;

            Ok(Response::builder()
                .status(200)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
        Err(err) => {
            let error_info = ErrorInfo::from(&err);
            tpl.error_message = Some(error_info.message);

            Ok(Response::builder()
                .status(error_info.status_code)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
    }
}

#[derive(Template)]
#[template(path = "widgets/orgs/select_owner.html")]
struct SelectOwnerTemplate {
    payload: NewOrgFormData,
    error_message: Option<String>,
}

async fn select_org_owner_handler(
    Extension(ctx): Extension<Ctx>,
    State(state): State<AppState>,
    Path(params): Path<UserParams>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::User, Action::Read)?;
    let token = create_csrf_token_svc("new_org", &state.config.jwt_secret)?;

    let mut tpl = SelectOwnerTemplate {
        payload: NewOrgFormData {
            token,
            name: "".to_string(),
            owner_id: 0,
            owner_email: "".to_string(),
        },
        error_message: None,
    };

    match get_user_svc(&state, &ctx, params.user_id).await {
        Ok(user) => {
            tpl.payload.owner_id = user.id;
            tpl.payload.owner_email = user.email;

            Ok(Response::builder()
                .status(200)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
        Err(err) => {
            let error_info = ErrorInfo::from(&err);
            tpl.error_message = Some(error_info.message);

            Ok(Response::builder()
                .status(error_info.status_code)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
    }
}

#[derive(Template)]
#[template(path = "pages/orgs/new.html")]
struct NewOrgTemplate {
    t: TemplateData,
    action: String,
    error_message: Option<String>,
}

#[derive(Template)]
#[template(path = "widgets/orgs/new_form.html")]
struct NewOrgFormTemplate {
    action: String,
    payload: NewOrgFormData,
    error_message: Option<String>,
}

async fn new_org_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::Org, Action::Create)?;

    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);
    t.title = String::from("Create New Org");

    let tpl = NewOrgTemplate {
        t,
        action: "/orgs/new".to_string(),
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

async fn post_new_org_handler(
    Extension(ctx): Extension<Ctx>,
    State(state): State<AppState>,
    Form(payload): Form<NewOrgFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::Org, Action::Create)?;

    let token = create_csrf_token_svc("new_org", &config.jwt_secret)?;

    let mut tpl = NewOrgFormTemplate {
        action: "/orgs/new".to_string(),
        payload: NewOrgFormData {
            name: "".to_string(),
            owner_id: 0,
            owner_email: "".to_string(),
            token,
        },
        error_message: None,
    };

    let status: StatusCode;

    let org = payload.clone();

    let result = create_org_svc(&state, &ctx, org).await;

    match result {
        Ok(_) => {
            let next_url = "/orgs".to_string();
            // Weird but can't do a redirect here, let htmx handle it
            return Ok(Response::builder()
                .status(200)
                .header("HX-Redirect", next_url)
                .body(Body::from("".to_string()))
                .context(ResponseBuilderSnafu)?);
        }
        Err(err) => {
            let error_info = ErrorInfo::from(&err);
            status = error_info.status_code;
            tpl.error_message = Some(error_info.message);
        }
    }

    tpl.payload.name = payload.name.clone();
    tpl.payload.owner_id = payload.owner_id;
    tpl.payload.owner_email = payload.owner_email.clone();

    // Will only arrive here on error
    Ok(Response::builder()
        .status(status)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "pages/orgs/view.html")]
struct OrgPageTemplate {
    t: TemplateData,
    org: OrgDto,
    updated: bool,
    can_edit: bool,
    can_delete: bool,
}

async fn org_page_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);

    t.title = format!("Org - {}", &org.name);

    let tpl = OrgPageTemplate {
        t,
        org,
        updated: false,
        can_edit: ctx.actor.has_permissions(&vec![Permission::OrgsEdit]),
        can_delete: ctx.actor.has_permissions(&vec![Permission::OrgsDelete]),
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/orgs/edit_controls.html")]
struct OrgControlsTemplate {
    org: OrgDto,
    updated: bool,
    can_edit: bool,
    can_delete: bool,
}

async fn org_controls_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::Org, Action::Update)?;

    let tpl = OrgControlsTemplate {
        org,
        updated: false,
        can_edit: ctx.actor.has_permissions(&vec![Permission::OrgsEdit]),
        can_delete: ctx.actor.has_permissions(&vec![Permission::OrgsDelete]),
    };

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/orgs/edit_form.html")]
struct EditOrgTemplate {
    org: OrgDto,
    payload: UpdateOrgFormData,
    error_message: Option<String>,
}

async fn edit_org_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::Org, Action::Update)?;
    let token = create_csrf_token_svc(org.id.to_string().as_str(), &config.jwt_secret)?;

    let mut status_opt = None;
    if &org.status == "active" {
        status_opt = Some("1".to_string());
    }

    let org_name = org.name.clone();
    let tpl = EditOrgTemplate {
        org,
        payload: UpdateOrgFormData {
            token,
            name: org_name,
            active: status_opt,
        },
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

async fn post_edit_org_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
    Form(payload): Form<UpdateOrgFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::Org, Action::Update)?;

    let token = create_csrf_token_svc(&org.id.to_string(), &config.jwt_secret)?;
    let org_id = org.id;

    let mut tpl = EditOrgTemplate {
        org,
        payload: UpdateOrgFormData {
            token,
            name: payload.name.clone(),
            active: payload.active.clone(),
        },
        error_message: None,
    };

    let result = update_org_svc(&state, &ctx, org_id, payload).await;

    match result {
        Ok(updated_org) => {
            // Render back the controls but when updated name and status
            let tpl = OrgControlsTemplate {
                org: updated_org,
                updated: true,
                can_edit: ctx.actor.has_permissions(&vec![Permission::OrgsEdit]),
                can_delete: ctx.actor.has_permissions(&vec![Permission::OrgsDelete]),
            };

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "text/html")
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
        Err(err) => {
            let status;
            match err {
                Error::Validation { msg } => {
                    status = StatusCode::BAD_REQUEST;
                    tpl.error_message = Some(msg);
                }
                Error::LoginRequired => {
                    status = StatusCode::UNAUTHORIZED;
                    tpl.error_message = Some("Login required.".to_string());
                }
                any_err => {
                    status = StatusCode::INTERNAL_SERVER_ERROR;
                    tpl.error_message = Some(any_err.to_string());
                }
            };

            Ok(Response::builder()
                .status(status)
                .header("Content-Type", "text/html")
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
    }
}

#[derive(Template)]
#[template(path = "widgets/orgs/search_new_owner.html")]
struct SearchNewOwnerTemplate {
    org_members: Vec<OrgMemberDto>,
    org_id: i32,
    error_message: Option<String>,
}

async fn search_new_org_owner_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
    Query(query): Query<ListOrgMembersParamsDto>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::OrgMember, Action::Read)?;

    let mut tpl = SearchNewOwnerTemplate {
        org_members: Vec::new(),
        org_id: org.id,
        error_message: None,
    };

    let mut updated_query = query.clone();
    updated_query.per_page = Some(10);

    match list_org_members_svc(&state, &ctx, org.id, updated_query).await {
        Ok(org_members) => {
            // Filter out the current owner from the list
            tpl.org_members = org_members
                .data
                .into_iter()
                .filter(|m| Some(m.user_id) != org.owner_id)
                .collect();

            Ok(Response::builder()
                .status(200)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
        Err(err) => {
            let error_info = ErrorInfo::from(&err);
            tpl.error_message = Some(error_info.message);

            Ok(Response::builder()
                .status(error_info.status_code)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
    }
}

#[derive(Template)]
#[template(path = "widgets/orgs/change_owner_form.html")]
struct ChangeOrgOwnerTemplate {
    org: OrgDto,
    error_message: Option<String>,
}

async fn change_org_owner_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::Org, Action::Update)?;

    let tpl = ChangeOrgOwnerTemplate {
        org,
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

async fn post_change_org_owner_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
    Form(payload): Form<UpdateOrgOwnerFormData>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::Org, Action::Update)?;
    let org_id = org.id;

    let mut tpl = ChangeOrgOwnerTemplate {
        org,
        error_message: None,
    };

    let result = update_org_owner_svc(&state, &ctx, org_id, payload).await;

    match result {
        Ok(updated_org) => {
            // Render back the controls but with updated name and status
            let tpl = OrgControlsTemplate {
                org: updated_org,
                updated: true,
                can_edit: ctx.actor.has_permissions(&vec![Permission::OrgsEdit]),
                can_delete: ctx.actor.has_permissions(&vec![Permission::OrgsDelete]),
            };

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "text/html")
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
        Err(err) => {
            let status;
            match err {
                Error::Validation { msg } => {
                    status = StatusCode::BAD_REQUEST;
                    tpl.error_message = Some(msg);
                }
                Error::LoginRequired => {
                    status = StatusCode::UNAUTHORIZED;
                    tpl.error_message = Some("Login required.".to_string());
                }
                any_err => {
                    status = StatusCode::INTERNAL_SERVER_ERROR;
                    tpl.error_message = Some(any_err.to_string());
                }
            };

            Ok(Response::builder()
                .status(status)
                .header("Content-Type", "text/html")
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
    }
}

#[derive(Template)]
#[template(path = "widgets/orgs/select_new_owner.html")]
struct SelectNewOwnerTemplate {
    org: OrgDto,
    payload: UpdateOrgOwnerFormData,
    error_message: Option<String>,
}

async fn select_new_org_owner_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
    Path(params): Path<UserParams>,
) -> Result<Response<Body>> {
    let _ = enforce_policy(&ctx.actor, Resource::OrgMember, Action::Read)?;
    let token = create_csrf_token_svc(org.id.to_string().as_str(), &state.config.jwt_secret)?;

    let org_id = org.id;
    let mut tpl = SelectNewOwnerTemplate {
        org,
        payload: UpdateOrgOwnerFormData {
            token,
            owner_id: 0,
            owner_email: "".to_string(),
        },
        error_message: None,
    };

    match get_org_member_svc(&state, &ctx, org_id, params.user_id).await {
        Ok(member) => {
            tpl.payload.owner_id = member.user_id;
            tpl.payload.owner_email = member.member_email.unwrap_or("".to_string());

            Ok(Response::builder()
                .status(200)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
        Err(err) => {
            let error_info = ErrorInfo::from(&err);
            tpl.error_message = Some(error_info.message);

            Ok(Response::builder()
                .status(error_info.status_code)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
    }
}

/*
#[derive(Template)]
#[template(path = "widgets/orgs/change_password_form.html")]
struct ChangePasswordTemplate {
    user: OrgDto,
    payload: ChangePasswordFormData,
    error_message: Option<String>,
}

async fn change_password_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(user): Extension<OrgDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::Org, Action::Update)?;
    let token = create_csrf_token_svc(&user.id.to_string(), &config.jwt_secret)?;

    let tpl = ChangePasswordTemplate {
        user,
        payload: ChangePasswordFormData {
            token,
            password: "".to_string(),
            confirm_password: "".to_string(),
        },
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

async fn post_change_password_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(user): Extension<OrgDto>,
    State(state): State<AppState>,
    payload: Form<ChangePasswordFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::Org, Action::Update)?;

    let token = create_csrf_token_svc(&user.id.to_string(), &config.jwt_secret)?;
    let user_id = user.id;

    let mut tpl = ChangePasswordTemplate {
        user: user.clone(),
        payload: ChangePasswordFormData {
            token,
            password: payload.password.clone(),
            confirm_password: payload.confirm_password.clone(),
        },
        error_message: None,
    };

    let data = ChangePasswordFormData {
        token: payload.token.clone(),
        password: payload.password.clone(),
        confirm_password: payload.confirm_password.clone(),
    };

    let result = change_user_password_svc(&state, &ctx, user_id, data).await;

    match result {
        Ok(_) => {
            let tpl = OrgControlsTemplate {
                user,
                updated: false,
                can_edit: ctx.actor.has_permissions(&vec![Permission::OrgsEdit]),
                can_delete: ctx.actor.has_permissions(&vec![Permission::OrgsDelete]),
            };

            Ok(Response::builder()
                .status(200)
                .header("Content-Type", "text/html")
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
        Err(err) => {
            let status;
            match err {
                Error::Validation { msg } => {
                    status = StatusCode::BAD_REQUEST;
                    tpl.error_message = Some(msg);
                }
                Error::LoginRequired => {
                    status = StatusCode::UNAUTHORIZED;
                    tpl.error_message = Some("Login required.".to_string());
                }
                any_err => {
                    status = StatusCode::INTERNAL_SERVER_ERROR;
                    tpl.error_message = Some(any_err.to_string());
                }
            };

            Ok(Response::builder()
                .status(status)
                .header("Content-Type", "text/html")
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
    }
}

#[derive(Template)]
#[template(path = "widgets/orgs/delete_form.html")]
struct DeleteOrgFormTemplate {
    user: OrgDto,
    payload: TokenFormData,
    error_message: Option<String>,
}

async fn delete_user_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(user): Extension<OrgDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::Org, Action::Delete)?;

    let token = create_csrf_token_svc(&user.id.to_string(), &config.jwt_secret)?;

    let tpl = DeleteOrgFormTemplate {
        user,
        payload: TokenFormData { token },
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

async fn post_delete_user_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(user): Extension<OrgDto>,
    State(state): State<AppState>,
    payload: Form<TokenFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    let _ = enforce_policy(&ctx.actor, Resource::Org, Action::Delete)?;

    let token = create_csrf_token_svc(&user.id.to_string(), &config.jwt_secret)?;

    let mut tpl = DeleteOrgFormTemplate {
        user: user.clone(),
        payload: TokenFormData { token },
        error_message: None,
    };

    let result = delete_user_svc(&state, &ctx, user.id, &payload.token).await;

    match result {
        Ok(_) => {
            // Render same form but trigger a redirect to home
            let tpl = DeleteOrgFormTemplate {
                user,
                payload: TokenFormData {
                    token: "".to_string(),
                },
                error_message: None,
            };
            return Ok(Response::builder()
                .status(200)
                .header("HX-Redirect", "/orgs".to_string())
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?);
        }
        Err(err) => {
            let error_info = ErrorInfo::from(&err);
            tpl.error_message = Some(error_info.message);

            Ok(Response::builder()
                .status(error_info.status_code)
                .body(Body::from(tpl.render().context(TemplateSnafu)?))
                .context(ResponseBuilderSnafu)?)
        }
    }
}
*/
