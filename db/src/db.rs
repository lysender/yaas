use deadpool_diesel::postgres::{Manager, Pool, Runtime};
use std::sync::Arc;

use crate::{
    app::{AppRepo, AppStore},
    oauth_code::{OauthCodeRepo, OauthCodeStore},
    org::{OrgRepo, OrgStore},
    org_app::{OrgAppRepo, OrgAppStore},
    org_member::{OrgMemberRepo, OrgMemberStore},
    password::{PasswordRepo, PasswordStore},
    superuser::{SuperuserRepo, SuperuserStore},
    user::{UserRepo, UserStore},
};

pub fn create_db_pool(database_url: &str) -> Pool {
    let manager = Manager::new(database_url, Runtime::Tokio1);
    Pool::builder(manager).max_size(8).build().unwrap()
}

pub struct DbMapper {
    pub apps: Arc<dyn AppStore>,
    pub oauth_codes: Arc<dyn OauthCodeStore>,
    pub orgs: Arc<dyn OrgStore>,
    pub org_apps: Arc<dyn OrgAppStore>,
    pub org_members: Arc<dyn OrgMemberStore>,
    pub passwords: Arc<dyn PasswordStore>,
    pub superusers: Arc<dyn SuperuserStore>,
    pub users: Arc<dyn UserStore>,
}

pub fn create_db_mapper(database_url: &str) -> DbMapper {
    let pool = create_db_pool(database_url);
    DbMapper {
        apps: Arc::new(AppRepo::new(pool.clone())),
        oauth_codes: Arc::new(OauthCodeRepo::new(pool.clone())),
        orgs: Arc::new(OrgRepo::new(pool.clone())),
        org_apps: Arc::new(OrgAppRepo::new(pool.clone())),
        org_members: Arc::new(OrgMemberRepo::new(pool.clone())),
        passwords: Arc::new(PasswordRepo::new(pool.clone())),
        superusers: Arc::new(SuperuserRepo::new(pool.clone())),
        users: Arc::new(UserRepo::new(pool.clone())),
    }
}

#[cfg(feature = "test")]
pub fn create_test_db_mapper() -> DbMapper {
    use crate::app::AppTestRepo;
    use crate::oauth_code::OauthCodeTestRepo;
    use crate::org::OrgTestRepo;
    use crate::org_app::OrgAppTestRepo;
    use crate::org_member::OrgMemberTestRepo;
    use crate::password::PasswordTestRepo;
    use crate::superuser::SuperuserTestRepo;
    use crate::user::UserTestRepo;

    DbMapper {
        apps: Arc::new(AppTestRepo {}),
        oauth_codes: Arc::new(OauthCodeTestRepo {}),
        orgs: Arc::new(OrgTestRepo {}),
        org_apps: Arc::new(OrgAppTestRepo {}),
        org_members: Arc::new(OrgMemberTestRepo {}),
        passwords: Arc::new(PasswordTestRepo {}),
        superusers: Arc::new(SuperuserTestRepo {}),
        users: Arc::new(UserTestRepo {}),
    }
}
