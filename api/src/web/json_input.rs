use axum::{Json, extract::rejection::JsonRejection};
use snafu::{ResultExt, ensure};
use validator::Validate;

use yaas::validators::flatten_errors;

use crate::{
    Result,
    error::{JsonRejectionSnafu, ValidationSnafu},
};

pub type JsonPayload<T> = std::result::Result<Json<T>, JsonRejection>;

pub fn parse_and_validate_json<T>(payload: JsonPayload<T>) -> Result<T>
where
    T: Validate,
{
    let Json(data) = payload.context(JsonRejectionSnafu {
        msg: "Invalid JSON payload".to_string(),
    })?;

    let errors = data.validate();
    ensure!(
        errors.is_ok(),
        ValidationSnafu {
            msg: flatten_errors(&errors.unwrap_err()),
        }
    );

    Ok(data)
}
