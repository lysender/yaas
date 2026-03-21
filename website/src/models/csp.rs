use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Deserialize, Serialize)]
pub struct CspNonce {
    pub nonce: String,
}

impl CspNonce {
    pub fn new() -> Self {
        let nonce = STANDARD.encode(Uuid::new_v4().as_bytes());
        Self { nonce }
    }
}
