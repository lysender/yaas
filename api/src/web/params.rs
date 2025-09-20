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
    pub org_id: i32,
}

#[derive(Deserialize)]
pub struct OrgMemberParams {
    #[allow(dead_code)]
    pub org_id: i32,

    pub org_member_id: i32,
}

#[derive(Deserialize)]
pub struct OrgAppParams {
    #[allow(dead_code)]
    pub org_id: i32,

    pub org_app_id: i32,
}
