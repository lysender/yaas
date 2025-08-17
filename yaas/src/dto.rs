use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct UserDto {
    pub id: String,
    pub email: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct SuperuserDto {
    pub id: String,
    pub created_at: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PasswordDto {
    pub id: String,
    pub password: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OrgDto {
    pub id: String,
    pub name: String,
    pub status: String,
    pub owner_id: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OrgMemberDto {
    pub id: String,
    pub org_id: String,
    pub user_id: String,
    pub roles: Vec<String>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AppDto {
    pub id: String,
    pub name: String,
    pub secret: String,
    pub redirect_uri: String,
    pub created_at: String,
    pub updated_at: String,
    pub deleted_at: Option<String>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OrgAppDto {
    pub id: String,
    pub org_id: String,
    pub app_id: String,
    pub created_at: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OauthCodeDto {
    pub id: String,
    pub code: String,
    pub state: String,
    pub redirect_uri: String,
    pub scope: String,
    pub app_id: String,
    pub org_id: String,
    pub user_id: String,
    pub created_at: String,
    pub expires_at: String,
}
