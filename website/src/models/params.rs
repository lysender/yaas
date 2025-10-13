use serde::Deserialize;

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
    pub org_id: String,
}

#[derive(Deserialize)]
pub struct OrgMemberParams {
    pub org_id: i32,
    pub user_id: i32,
}

#[derive(Deserialize)]
pub struct OrgAppParams {
    pub org_id: i32,
    pub app_id: i32,
}
