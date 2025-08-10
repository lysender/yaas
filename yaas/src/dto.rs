pub struct UserDto {
    pub id: String,
    pub email: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct PasswordDto {
    pub id: String,
    pub password: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct OrgDto {
    pub id: String,
    pub name: String,
    pub status: String,
    pub owner_id: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct OrgMemberDto {
    pub id: String,
    pub org_id: String,
    pub user_id: String,
    pub roles: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct AppDto {
    pub id: String,
    pub name: String,
    pub secret: String,
    pub redirect_uri: String,
    pub created_at: String,
    pub updated_at: String,
}

pub struct OrgAppDto {
    pub id: String,
    pub org_id: String,
    pub app_id: String,
    pub created_at: String,
}

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
    pub updated_at: String,
}
