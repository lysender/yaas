use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::buffed::dto::{SetupBodyBuf, SuperuserBuf};

#[derive(Clone, Serialize, Deserialize)]
pub struct SuperuserDto {
    pub id: i32,
    pub created_at: String,
}

impl From<SuperuserBuf> for SuperuserDto {
    fn from(su: SuperuserBuf) -> Self {
        SuperuserDto {
            id: su.id,
            created_at: su.created_at,
        }
    }
}

#[derive(Clone, Serialize, Deserialize, Validate)]
pub struct SetupBodyDto {
    #[validate(length(equal = 36))]
    pub setup_key: String,

    #[validate(length(max = 100))]
    #[validate(email)]
    pub email: String,

    #[validate(length(min = 8, max = 60))]
    pub password: String,
}

impl From<SetupBodyBuf> for SetupBodyDto {
    fn from(body: SetupBodyBuf) -> Self {
        SetupBodyDto {
            setup_key: body.setup_key,
            email: body.email,
            password: body.password,
        }
    }
}
