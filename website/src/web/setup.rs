use askama::Template;
use axum::{
    body::Body,
    extract::{Form, Query, State},
    http::Response,
    response::{IntoResponse, Redirect},
};
use snafu::ResultExt;
use std::collections::HashMap;
use validator::Validate;

use crate::{
    Error, Result,
    error::{ResponseBuilderSnafu, TemplateSnafu},
    models::{SetupFormPayload, TemplateData},
    run::AppState,
    services::setup_superuser_svc,
};
use yaas::{dto::Actor, validators::flatten_errors};

use crate::models::Pref;

#[derive(Template)]
#[template(path = "pages/setup.html")]
struct SetupTemplate {
    t: TemplateData,
    error_message: Option<String>,
    success_message: Option<String>,
}

pub async fn setup_handler(
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Response<Body>> {
    let pref = Pref::new();
    let actor = Actor::default();
    let mut t = TemplateData::new(&state, actor, &pref);
    t.title = String::from("Yaas Setup");

    let error_message = query.get("error").cloned();
    let success_message = match query.get("success") {
        Some(val) if val == "1" => Some("Setup complete. Superuser may now login.".to_string()),
        _ => None,
    };

    let tpl = SetupTemplate {
        t,
        error_message,
        success_message,
    };

    Response::builder()
        .status(200)
        .header("Surrogate-Control", "no-store")
        .header(
            "Cache-Control",
            "no-store, no-cache, must-revalidate, proxy-revalidate",
        )
        .header("Pragma", "no-cache")
        .header("Expires", 0)
        .body(Body::from(tpl.render().context(TemplateSnafu)?))
        .context(ResponseBuilderSnafu)
}

pub async fn post_setup_handler(
    State(state): State<AppState>,
    Form(payload): Form<SetupFormPayload>,
) -> impl IntoResponse {
    if let Err(err) = payload.validate() {
        let msg = flatten_errors(&err);
        return handle_error(Error::Validation { msg });
    }

    if payload.password != payload.password_confirm {
        return handle_error(Error::Validation {
            msg: "Password and repeat password must match".to_string(),
        });
    }

    if payload.setup_key.trim().is_empty() {
        return handle_error(Error::Validation {
            msg: "Setup key must not be empty".to_string(),
        });
    }

    let result = setup_superuser_svc(&state, payload.setup_key, payload.email, payload.password).await;
    match result {
        Ok(_) => Redirect::to("/setup?success=1").into_response(),
        Err(err) => handle_error(err),
    }
}

fn handle_error(error: Error) -> Response<Body> {
    let url = format!(
        "/setup?error={}",
        urlencoding::encode(&error.to_string())
    );
    Redirect::to(&url).into_response()
}
