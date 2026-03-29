use reqwest::StatusCode;
use snafu::ResultExt;
use yaas::dto::{SetupBodyDto, SetupStatusDto};

use crate::{
    Error, Result,
    error::{HttpClientSnafu, HttpResponseParseSnafu},
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
    let body = SetupBodyDto {
        setup_key,
        email,
        password,
    };

    let response = state
        .client
        .post(url)
        .json(&body)
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
            let setup_status =
                response
                    .json::<SetupStatusDto>()
                    .await
                    .context(HttpResponseParseSnafu {
                        msg: "Unable to parse setup status response.".to_string(),
                    })?;
            Ok(setup_status.done)
        }
        _ => Err(handle_response_error(
            response,
            "setup",
            Error::Service {
                msg: "Unable to process setup status request. Try again later.".to_string(),
            },
        )
        .await),
    }
}
