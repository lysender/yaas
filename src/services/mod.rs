mod apps;
pub mod auth;
pub mod captcha;
mod oauth;
mod org_apps;
mod org_members;
mod orgs;
mod setup;
pub mod token;
pub mod users;

use reqwest::StatusCode;
use snafu::ResultExt;
use yaas::dto::ErrorMessageDto;

use crate::{Error, Result, error::HttpResponseParseSnafu};

pub use apps::*;
pub use oauth::*;
pub use org_apps::*;
pub use org_members::*;
pub use orgs::*;
pub use setup::*;

pub async fn handle_response_error(
    response: reqwest::Response,
    resource: &str,
    default_error: Error,
) -> Error {
    // Assumes that ok responses are already handled
    let status = response.status();
    let message_res = parse_response_error(response).await;
    match message_res {
        Ok(msg) => match status {
            StatusCode::BAD_REQUEST => Error::BadRequest { msg },
            StatusCode::UNAUTHORIZED => Error::LoginRequired,
            StatusCode::FORBIDDEN => Error::Forbidden {
                msg: format!("You have no permissions to view {}", resource),
            },
            StatusCode::NOT_FOUND => default_error,
            _ => Error::Service {
                msg: "Service error. Try again later.".to_string(),
            },
        },
        Err(err) => err,
    }
}

pub async fn parse_response_error(response: reqwest::Response) -> Result<String> {
    let content_type = response
        .headers()
        .get("Content-Type")
        .and_then(|header| header.to_str().ok())
        .unwrap_or("");

    if content_type.starts_with("application/json") {
        let error = response
            .json::<ErrorMessageDto>()
            .await
            .context(HttpResponseParseSnafu {
                msg: "Unable to parse JSON error response.".to_string(),
            })?;
        return Ok(error.message);
    }

    let text = response.text().await.context(HttpResponseParseSnafu {
        msg: "Unable to parse service error response.".to_string(),
    })?;

    if let Ok(error) = serde_json::from_str::<ErrorMessageDto>(&text) {
        return Ok(error.message);
    }

    if !text.is_empty() {
        return Ok(text);
    }

    Err(Error::Service {
        msg: "Unable to parse service error response".to_string(),
    })
}
