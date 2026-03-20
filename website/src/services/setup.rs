use prost::Message;
use reqwest::StatusCode;
use snafu::ResultExt;
use yaas::buffed::dto::{SetupBodyBuf, SetupStatusBuf};

use crate::{
    Error, Result,
    error::{HttpClientSnafu, HttpResponseBytesSnafu, ProtobufDecodeSnafu},
    run::AppState,
    services::handle_response_error,
};

pub async fn setup_superuser_svc(
    state: &AppState,
    setup_key: String,
    email: String,
    password: String,
) -> Result<()> {
    let url = format!("{}/setup", &state.config.api_url);
    let body = SetupBodyBuf {
        setup_key,
        email,
        password,
    };

    let response = state
        .client
        .post(url)
        .body(prost::Message::encode_to_vec(&body))
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to process setup request. Try again later.".to_string(),
        })?;

    if !response.status().is_success() {
        return Err(handle_response_error(response, "setup", Error::LoginFailed).await);
    }

    Ok(())
}

pub async fn setup_status_svc(state: &AppState) -> Result<bool> {
    let url = format!("{}/setup", &state.config.api_url);

    let response = state
        .client
        .get(url)
        .send()
        .await
        .context(HttpClientSnafu {
            msg: "Unable to process setup status request. Try again later.".to_string(),
        })?;

    match response.status() {
        StatusCode::OK => {
            let body_bytes = response.bytes().await.context(HttpResponseBytesSnafu {})?;
            let setup_status = SetupStatusBuf::decode(&body_bytes[..]).context(ProtobufDecodeSnafu {})?;
            Ok(setup_status.done)
        }
        _ => Err(handle_response_error(response, "setup", Error::Service {
            msg: "Unable to process setup status request. Try again later.".to_string(),
        })
        .await),
    }
}
