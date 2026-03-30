use askama::Template;
use axum::{
    Extension,
    body::Body,
    extract::{Form, Query, State},
    http::Response,
    response::{IntoResponse, Redirect},
};
use snafu::ResultExt;
use std::collections::HashMap;
use urlencoding::encode;
use validator::Validate;

use crate::{
    Error, Result,
    error::{ErrorInfo, ResponseBuilderSnafu, TemplateSnafu},
    models::{CspNonce, SetupFormPayload, TemplateData},
    run::AppState,
    services::{setup_status_svc, setup_superuser_svc},
    web::handle_error,
};
use crate::{dto::Actor, validators::flatten_errors};

use crate::models::Pref;

#[derive(Template)]
#[template(path = "pages/setup.html")]
struct SetupTemplate {
    t: TemplateData,
    error_message: Option<String>,
}

pub async fn setup_handler(
    Extension(csp_nonce): Extension<CspNonce>,
    State(state): State<AppState>,
    Query(query): Query<HashMap<String, String>>,
) -> Result<Response<Body>> {
    let setup_done = setup_status_svc(&state).await?;
    if setup_done {
        let not_found = ErrorInfo {
            status_code: axum::http::StatusCode::NOT_FOUND,
            title: String::from("Not Found"),
            message: String::from("The page you are looking for cannot be found."),
        };
        return Ok(handle_error(
            &state,
            Actor::default(),
            &Pref::new(),
            csp_nonce.nonce,
            not_found,
            true,
        ));
    }

    let pref = Pref::new();
    let actor = Actor::default();
    let mut t = TemplateData::new(&state, actor, &pref, csp_nonce.nonce);
    t.title = String::from("Yaas Setup");

    let error_message = query.get("error").cloned();

    let tpl = SetupTemplate { t, error_message };

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
    Extension(csp_nonce): Extension<CspNonce>,
    State(state): State<AppState>,
    Form(payload): Form<SetupFormPayload>,
) -> impl IntoResponse {
    let setup_done = match setup_status_svc(&state).await {
        Ok(done) => done,
        Err(err) => return handle_submit_error(err),
    };

    if setup_done {
        let not_found = ErrorInfo {
            status_code: axum::http::StatusCode::NOT_FOUND,
            title: String::from("Not Found"),
            message: String::from("The page you are looking for cannot be found."),
        };
        return handle_error(
            &state,
            Actor::default(),
            &Pref::new(),
            csp_nonce.nonce,
            not_found,
            true,
        );
    }

    if let Err(err) = payload.validate() {
        let msg = flatten_errors(&err);
        return handle_submit_error(Error::Validation { msg });
    }

    if payload.password != payload.password_confirm {
        return handle_submit_error(Error::Validation {
            msg: "Password and repeat password must match".to_string(),
        });
    }

    if payload.setup_key.trim().is_empty() {
        return handle_submit_error(Error::Validation {
            msg: "Setup key must not be empty".to_string(),
        });
    }

    let result =
        setup_superuser_svc(&state, payload.setup_key, payload.email, payload.password).await;

    match result {
        Ok(_) => {
            let url = format!(
                "/login?success={}",
                encode("Setup complete. Login with superuser credentials.")
            );
            Redirect::to(&url).into_response()
        }
        Err(err) => handle_submit_error(err),
    }
}

fn handle_submit_error(error: Error) -> Response<Body> {
    let url = format!("/setup?error={}", urlencoding::encode(&error.to_string()));
    Redirect::to(&url).into_response()
}
