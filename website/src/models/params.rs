use core::fmt;
use urlencoding::encode;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct UserParams {
    pub user_id: String,
}
