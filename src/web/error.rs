use askama::Template;
use axum::{Extension, body::Body, extract::State, http::StatusCode, response::Response};

use crate::dto::Actor;
use crate::{
    Error,
    error::ErrorInfo,
    models::{CspNonce, Pref, TemplateData},
    run::AppState,
};

#[derive(Clone, Template)]
#[template(path = "pages/error.html")]
struct ErrorPageData {
    t: TemplateData,
    error: ErrorInfo,
}

#[derive(Clone, Template)]
#[template(path = "widgets/error.html")]
struct ErrorWidgetData {
    error: ErrorInfo,
}

#[derive(Clone, Template)]
#[template(path = "widgets/error_message.html")]
struct ErrorMessageData {
    message: String,
}

pub async fn error_handler(
    Extension(csp_nonce): Extension<CspNonce>,
    State(state): State<AppState>,
) -> Response<Body> {
    let actor = Actor::default();
    let pref = Pref::new();

    handle_error(
        &state,
        actor,
        &pref,
        csp_nonce.nonce,
        ErrorInfo {
            status_code: StatusCode::NOT_FOUND,
            title: String::from("Not Found"),
            message: String::from("The page you are looking for cannot be found."),
        },
        true,
    )
}

/// Render an error page or an error widget
pub fn handle_error(
    state: &AppState,
    actor: Actor,
    pref: &Pref,
    nonce: String,
    error: ErrorInfo,
    full_page: bool,
) -> Response<Body> {
    if full_page {
        let title = error.title.as_str();
        let status_code = error.status_code;

        let mut t = TemplateData::new(state, actor, pref, nonce);
        t.title = String::from(title);

        let tpl = ErrorPageData { t, error };

        Response::builder()
            .status(status_code)
            .header("Content-Type", "text/html; charset=utf-8")
            .body(Body::from(
                tpl.render().expect("Error template must render"),
            ))
            .expect("Response builder must succeed")
    } else {
        let status_code = error.status_code;
        let tpl = ErrorWidgetData { error };

        Response::builder()
            .status(status_code)
            .header("Content-Type", "text/html; charset=utf-8")
            .body(Body::from(
                tpl.render().expect("Error template must render"),
            ))
            .expect("Response builder must succeed")
    }
}

/// Render a simple error message
pub fn handle_error_message(error: &Error) -> Response<Body> {
    let error_info: ErrorInfo = error.into();
    let tpl = ErrorMessageData {
        message: error_info.message,
    };

    Response::builder()
        .status(error_info.status_code)
        .header("Content-Type", "text/html; charset=utf-8")
        .body(Body::from(
            tpl.render().expect("Error template must render"),
        ))
        .expect("Response builder must succeed")
}
