use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct ErrorMessageDto {
    pub status_code: u16,
    pub message: String,
    pub error: String,
    pub error_code: Option<String>,
}
