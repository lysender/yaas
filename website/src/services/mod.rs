mod apps;
pub mod auth;
pub mod captcha;
mod oauth;
mod org_apps;
mod org_members;
mod orgs;
pub mod token;
pub mod users;

use prost::Message;
use reqwest::StatusCode;
use snafu::ResultExt;
use yaas::buffed::dto::ErrorMessageBuf;

use crate::{
    Error, Result,
    error::{ErrorResponse, HttpResponseBytesSnafu, HttpResponseParseSnafu, ProtobufDecodeSnafu},
};

pub use apps::*;
pub use oauth::*;
pub use org_apps::*;
pub use org_members::*;
pub use orgs::*;

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
    let Some(content_type) = response.headers().get("Content-Type") else {
        return Err(Error::Service {
            msg: "Unable to identify service response type".to_string(),
        });
    };

    let Ok(content_type) = content_type.to_str() else {
        return Err(Error::Service {
            msg: "Unable to identify service response type".to_string(),
        });
    };

    match content_type {
        "application/x-protobuf" => {
            let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
            let msg = ErrorMessageBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;
            Ok(msg.message)
        }
        "application/json" => {
            // Expected response when properly handled by the backend service
            let json = response
                .json::<ErrorResponse>()
                .await
                .context(HttpResponseParseSnafu {
                    msg: "Unable to parse error response.",
                })?;
            Ok(json.message)
        }
        "text/plain" | "text/plain; charset=utf-8" => {
            // Probably some default http error
            let text_res = response.text().await;
            match text_res {
                Ok(text) => Ok(text),
                Err(_) => Err(Error::Service {
                    msg: "Unable to parse text service error response".to_string(),
                }),
            }
        }
        _ => Err(Error::Service {
            msg: "Unable to parse service error response".to_string(),
        }),
    }
}
