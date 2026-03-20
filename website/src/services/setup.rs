use snafu::ResultExt;
use yaas::buffed::dto::SetupBodyBuf;

use crate::{
    Error, Result,
    error::HttpClientSnafu,
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
