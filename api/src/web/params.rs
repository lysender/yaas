use serde::Deserialize;

#[derive(Deserialize)]
pub struct Params {
    pub bucket_id: String,
    pub dir_id: Option<String>,
    pub file_id: Option<String>,
}

#[derive(Deserialize)]
pub struct ClientParams {
    pub client_id: String,
}

#[derive(Deserialize)]
pub struct UserParams {
    pub user_id: i32,
}

#[derive(Deserialize)]
pub struct AppParams {
    pub app_id: i32,
}

#[derive(Deserialize)]
pub struct OrgParams {
    pub org_id: i32,
}

#[derive(Deserialize)]
pub struct OrgMemberParams {
    pub org_id: i32,
    pub org_member_id: i32,
}

#[derive(Deserialize)]
pub struct OrgAppParams {
    pub org_id: i32,
    pub org_app_id: i32,
}
