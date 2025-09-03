use serde::{Deserialize, Serialize};

use crate::buffed::dto::ErrorMessageBuf;

#[derive(Clone, Serialize, Deserialize)]
pub struct ErrorMessageDto {
    pub status_code: u16,
    pub message: String,
    pub error: String,
}

impl From<ErrorMessageBuf> for ErrorMessageDto {
    fn from(err: ErrorMessageBuf) -> Self {
        ErrorMessageDto {
            status_code: err.status_code as u16,
            message: err.message,
            error: err.error,
        }
    }
}
