use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
pub struct Params {
    pub bucket_id: String,
    pub dir_id: Option<String>,
    pub file_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ClientParams {
    pub client_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct UserParams {
    pub user_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct AppParams {
    pub app_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OrgParams {
    pub org_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OrgMemberParams {
    pub org_id: String,
    pub org_member_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct OrgAppParams {
    pub org_id: String,
    pub org_app_id: String,
}
