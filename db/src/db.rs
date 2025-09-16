use deadpool_diesel::postgres::{Manager, Pool, Runtime};

use crate::{
    app::AppRepo, oauth_code::OauthCodeRepo, org::OrgRepo, org_app::OrgAppRepo,
    org_member::OrgMemberRepo, password::PasswordRepo, superuser::SuperuserRepo, user::UserRepo,
};

pub fn create_db_pool(database_url: &str) -> Pool {
    let manager = Manager::new(database_url, Runtime::Tokio1);
    Pool::builder(manager).max_size(8).build().unwrap()
}

pub struct DbMapper {
    pub apps: AppRepo,
    pub oauth_codes: OauthCodeRepo,
    pub orgs: OrgRepo,
    pub org_apps: OrgAppRepo,
    pub org_members: OrgMemberRepo,
    pub passwords: PasswordRepo,
    pub superusers: SuperuserRepo,
    pub users: UserRepo,
}

pub fn create_db_mapper(database_url: &str) -> DbMapper {
    let pool = create_db_pool(database_url);
    DbMapper {
        apps: AppRepo::new(pool.clone()),
        oauth_codes: OauthCodeRepo::new(pool.clone()),
        orgs: OrgRepo::new(pool.clone()),
        org_apps: OrgAppRepo::new(pool.clone()),
        org_members: OrgMemberRepo::new(pool.clone()),
        passwords: PasswordRepo::new(pool.clone()),
        superusers: SuperuserRepo::new(pool.clone()),
        users: UserRepo::new(pool.clone()),
    }
}
