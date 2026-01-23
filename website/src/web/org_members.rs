use askama::Template;
use axum::extract::{Path, Query};
use axum::http::StatusCode;
use axum::{Extension, Form, body::Body, extract::State, response::Response};
use axum::{Router, middleware, routing::get};
use snafu::{ResultExt, ensure};
use urlencoding::encode;
use validator::Validate;

use crate::error::ValidationSnafu;
use crate::models::options::SelectOption;
use crate::models::{OrgMemberParams, PaginationLinks, TokenFormData};
use crate::services::users::get_user_svc;
use crate::services::{
    NewOrgMemberFormData, UpdateOrgMemberFormData, create_org_member_svc, delete_org_member_svc,
    list_org_member_suggestions_svc, list_org_members_svc, update_org_member_svc,
};
use crate::web::middleware::org_member_middleware;
use crate::{
    Error, Result,
    ctx::Ctx,
    error::{ErrorInfo, ResponseBuilderSnafu, TemplateSnafu},
    models::{Pref, TemplateData},
    run::AppState,
    services::token::create_csrf_token_svc,
    web::{Action, Resource, enforce_policy},
};
use yaas::dto::{ListOrgMembersParamsDto, OrgMemberSuggestionDto};
use yaas::dto::{OrgDto, OrgMemberDto};
use yaas::role::{Permission, Role};
use yaas::validators::flatten_errors;

pub fn org_members_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(org_members_handler))
        .route("/search", get(search_org_members_handler))
        .route(
            "/new",
            get(new_org_member_handler).post(post_new_org_member_handler),
        )
        .route(
            "/member-suggestions",
            get(search_member_suggestions_handler),
        )
        .route(
            "/select-member-suggestion/{user_id}",
            get(select_org_member_suggestion_handler),
        )
        .nest("/{user_id}", org_member_inner_routes(state.clone()))
        .with_state(state)
}

fn org_member_inner_routes(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/", get(org_member_page_handler))
        .route("/edit-controls", get(org_member_controls_handler))
        .route(
            "/edit",
            get(update_org_member_handler).post(post_update_org_member_handler),
        )
        .route(
            "/delete",
            get(delete_org_member_handler).post(post_delete_org_member_handler),
        )
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            org_member_middleware,
        ))
        .with_state(state)
}

#[derive(Template)]
#[template(path = "pages/org_members/index.html")]
struct OrgMembersPageTemplate {
    t: TemplateData,
    org: OrgDto,
    query_params: String,
}

async fn org_members_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
    Query(query): Query<ListOrgMembersParamsDto>,
) -> Result<Response<Body>> {
    enforce_policy(&ctx.actor, Resource::OrgMember, Action::Read)?;

    let errors = query.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);
    t.title = String::from("Organization Members");

    let tpl = OrgMembersPageTemplate {
        t,
        org,
        query_params: query.to_string(),
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/org_members/search.html")]
struct SearchOrgMembersTemplate {
    org_members: Vec<OrgMemberDto>,
    pagination: Option<PaginationLinks>,
    error_message: Option<String>,
}
async fn search_org_members_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
    Query(query): Query<ListOrgMembersParamsDto>,
) -> Result<Response<Body>> {
    enforce_policy(&ctx.actor, Resource::OrgMember, Action::Read)?;

    let mut tpl = SearchOrgMembersTemplate {
        org_members: Vec::new(),
        pagination: None,
        error_message: None,
    };

    let keyword = query.keyword.clone();

    match list_org_members_svc(&state, &ctx, org.id, query).await {
        Ok(org_members) => {
            let mut keyword_param: String = "".to_string();
            if let Some(keyword) = &keyword {
                keyword_param = format!("&keyword={}", encode(keyword).to_string());
            }
            tpl.org_members = org_members.data;
            tpl.pagination = Some(PaginationLinks::new(
                &org_members.meta,
                format!("/orgs/{}/members/search", org.id).as_str(),
                format!("/orgs/{}/members", org.id).as_str(),
                &keyword_param,
                ".org-members",
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
#[template(path = "widgets/org_members/search_member_suggestions.html")]
struct SearchMemberSuggestionsTemplate {
    org: OrgDto,
    suggestions: Vec<OrgMemberSuggestionDto>,
    error_message: Option<String>,
}

async fn search_member_suggestions_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
    Query(query): Query<ListOrgMembersParamsDto>,
) -> Result<Response<Body>> {
    enforce_policy(&ctx.actor, Resource::User, Action::Read)?;

    let org_id = org.id;
    let mut tpl = SearchMemberSuggestionsTemplate {
        org,
        suggestions: Vec::new(),
        error_message: None,
    };

    let mut updated_query = query.clone();
    updated_query.per_page = Some(10);

    match list_org_member_suggestions_svc(&state, &ctx, org_id, updated_query).await {
        Ok(users) => {
            tpl.suggestions = users.data;

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
#[template(path = "widgets/org_members/select_member_suggestion.html")]
struct SelectMemberSuggestionTemplate {
    org: OrgDto,
    payload: NewOrgMemberFormData,
    role_options: Vec<SelectOption>,
    error_message: Option<String>,
}

fn create_role_options() -> Vec<SelectOption> {
    vec![
        SelectOption {
            value: Role::OrgAdmin.to_string(),
            label: "Admin".to_string(),
        },
        SelectOption {
            value: Role::OrgEditor.to_string(),
            label: "Editor".to_string(),
        },
        SelectOption {
            value: Role::OrgViewer.to_string(),
            label: "Viewer".to_string(),
        },
    ]
}

async fn select_org_member_suggestion_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
    Path(params): Path<OrgMemberParams>,
) -> Result<Response<Body>> {
    enforce_policy(&ctx.actor, Resource::User, Action::Read)?;
    let token = create_csrf_token_svc("new_org_member", &state.config.jwt_secret)?;

    let mut tpl = SelectMemberSuggestionTemplate {
        org,
        payload: NewOrgMemberFormData {
            token,
            user_id: 0,
            user_email: "".to_string(),
            role: "".to_string(),
            active: Some("1".to_string()),
        },
        role_options: create_role_options(),
        error_message: None,
    };

    match get_user_svc(&state, &ctx, params.user_id).await {
        Ok(user) => {
            tpl.payload.user_id = user.id;
            tpl.payload.user_email = user.email;

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
#[template(path = "pages/org_members/new.html")]
struct NewOrgMemberTemplate {
    t: TemplateData,
    action: String,
    org: OrgDto,
    error_message: Option<String>,
}

#[derive(Template)]
#[template(path = "widgets/org_members/new_form.html")]
struct NewOrgMemberFormTemplate {
    action: String,
    org: OrgDto,
    payload: NewOrgMemberFormData,
    error_message: Option<String>,
}

async fn new_org_member_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    enforce_policy(&ctx.actor, Resource::OrgMember, Action::Create)?;

    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);
    t.title = String::from("Create New Org Member");

    let tpl = NewOrgMemberTemplate {
        t,
        action: format!("/orgs/{}/members/new", org.id),
        org,
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

async fn post_new_org_member_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    State(state): State<AppState>,
    Form(payload): Form<NewOrgMemberFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    enforce_policy(&ctx.actor, Resource::OrgMember, Action::Create)?;

    let org_id = org.id;
    let token = create_csrf_token_svc("new_org_member", &config.jwt_secret)?;

    let mut tpl = NewOrgMemberFormTemplate {
        action: format!("/orgs/{}/members/new", org_id),
        org,
        payload: NewOrgMemberFormData {
            token,
            user_id: payload.user_id,
            user_email: payload.user_email.clone(),
            role: payload.role.clone(),
            active: payload.active.clone(),
        },
        error_message: None,
    };

    let status: StatusCode;

    let result = create_org_member_svc(&state, &ctx, org_id, payload).await;

    match result {
        Ok(_) => {
            let next_url = format!("/orgs/{}/members", org_id);
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

    // Will only arrive here on error
    Ok(Response::builder()
        .status(status)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "pages/org_members/view.html")]
struct OrgMemberPageTemplate {
    t: TemplateData,
    org: OrgDto,
    org_member: OrgMemberDto,
    updated: bool,
    can_edit: bool,
    can_delete: bool,
}

async fn org_member_page_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(pref): Extension<Pref>,
    Extension(org): Extension<OrgDto>,
    Extension(org_member): Extension<OrgMemberDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let mut t = TemplateData::new(&state, ctx.actor.clone(), &pref);
    let member_email = org_member.member_email.clone().unwrap_or("".to_string());

    t.title = format!("Org Member - {}", member_email,);

    let tpl = OrgMemberPageTemplate {
        t,
        org,
        org_member,
        updated: false,
        can_edit: ctx.actor.has_permissions(&vec![Permission::OrgMembersEdit]),
        can_delete: ctx
            .actor
            .has_permissions(&vec![Permission::OrgMembersDelete]),
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/org_members/edit_controls.html")]
struct OrgMemberControlsTemplate {
    org_member: OrgMemberDto,
    updated: bool,
    can_edit: bool,
    can_delete: bool,
}

async fn org_member_controls_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org_member): Extension<OrgMemberDto>,
) -> Result<Response<Body>> {
    enforce_policy(&ctx.actor, Resource::OrgMember, Action::Update)?;

    let tpl = OrgMemberControlsTemplate {
        org_member,
        updated: false,
        can_edit: ctx.actor.has_permissions(&vec![Permission::OrgMembersEdit]),
        can_delete: ctx
            .actor
            .has_permissions(&vec![Permission::OrgMembersDelete]),
    };

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

#[derive(Template)]
#[template(path = "widgets/org_members/edit_form.html")]
struct UpdateOrgMemberTemplate {
    org_member: OrgMemberDto,
    payload: UpdateOrgMemberFormData,
    role_options: Vec<SelectOption>,
    error_message: Option<String>,
}

async fn update_org_member_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org_member): Extension<OrgMemberDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    enforce_policy(&ctx.actor, Resource::OrgMember, Action::Update)?;
    let token = create_csrf_token_svc(org_member.user_id.to_string().as_str(), &config.jwt_secret)?;

    // We only expect one role
    let role = org_member.roles.first().unwrap().to_string();
    let active = match org_member.status.as_str() {
        "active" => Some("1".to_string()),
        _ => None,
    };

    let tpl = UpdateOrgMemberTemplate {
        org_member,
        payload: UpdateOrgMemberFormData {
            token,
            role,
            active,
        },
        role_options: create_role_options(),
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .header("Content-Type", "text/html")
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

async fn post_update_org_member_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org_member): Extension<OrgMemberDto>,
    State(state): State<AppState>,
    Form(payload): Form<UpdateOrgMemberFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    enforce_policy(&ctx.actor, Resource::OrgMember, Action::Update)?;

    let token = create_csrf_token_svc(&org_member.user_id.to_string(), &config.jwt_secret)?;
    let org_id = org_member.org_id;
    let user_id = org_member.user_id;

    // We only expect one role
    let role = org_member.roles.first().unwrap().to_string();
    let active = match org_member.status.as_str() {
        "active" => Some("1".to_string()),
        _ => None,
    };

    let mut tpl = UpdateOrgMemberTemplate {
        org_member,
        payload: UpdateOrgMemberFormData {
            token,
            role,
            active,
        },
        role_options: create_role_options(),
        error_message: None,
    };

    let result = update_org_member_svc(&state, &ctx, org_id, user_id, payload).await;

    match result {
        Ok(updated_member) => {
            // Render back the controls but with updated data
            let tpl = OrgMemberControlsTemplate {
                org_member: updated_member,
                updated: true,
                can_edit: ctx.actor.has_permissions(&vec![Permission::OrgMembersEdit]),
                can_delete: ctx
                    .actor
                    .has_permissions(&vec![Permission::OrgMembersDelete]),
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
#[template(path = "widgets/org_members/delete_form.html")]
struct DeleteOrgMemberFormTemplate {
    org_member: OrgMemberDto,
    payload: TokenFormData,
    error_message: Option<String>,
}

async fn delete_org_member_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org_member): Extension<OrgMemberDto>,
    State(state): State<AppState>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    enforce_policy(&ctx.actor, Resource::OrgMember, Action::Delete)?;

    let token = create_csrf_token_svc(&org_member.user_id.to_string(), &config.jwt_secret)?;

    let tpl = DeleteOrgMemberFormTemplate {
        org_member,
        payload: TokenFormData { token },
        error_message: None,
    };

    Ok(Response::builder()
        .status(200)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)?)
}

async fn post_delete_org_member_handler(
    Extension(ctx): Extension<Ctx>,
    Extension(org): Extension<OrgDto>,
    Extension(org_member): Extension<OrgMemberDto>,
    State(state): State<AppState>,
    payload: Form<TokenFormData>,
) -> Result<Response<Body>> {
    let config = state.config.clone();

    enforce_policy(&ctx.actor, Resource::OrgMember, Action::Delete)?;

    let token = create_csrf_token_svc(&org_member.user_id.to_string(), &config.jwt_secret)?;
    let org_id = org.id;
    let user_id = org_member.user_id;

    let mut tpl = DeleteOrgMemberFormTemplate {
        org_member,
        payload: TokenFormData { token },
        error_message: None,
    };

    let result = delete_org_member_svc(&state, &ctx, org_id, user_id, &payload.token).await;

    match result {
        Ok(_) => {
            // Render same form but trigger a redirect to home
            return Ok(Response::builder()
                .status(200)
                .header("HX-Redirect", format!("/orgs/{}/members", org_id))
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
