use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use moka::sync::Cache;
use reqwest::ClientBuilder;
use snafu::ResultExt;
use turso::{Builder, Connection, Value};

use crate::Result;
use crate::config::{AssetManifest, Config, DbConfig, ServerConfig, SuperuserConfig};
use crate::ctx::Ctx;
use crate::db::create_db_mapper;
use crate::dto::{
    Actor, ActorPayloadDto, AppDto, NewAppDto, NewOrgAppDto, NewOrgDto, NewUserWithPasswordDto,
    OrgDto, Role, Scope, UserDto,
};
use crate::error::{DbBuilderSnafu, DbConnectSnafu, DbPrepareSnafu, DbStatementSnafu, IoSnafu};
use crate::run::AppState;
use crate::services::apps::create_app_svc;
use crate::services::org_apps::create_org_app_svc;
use crate::services::orgs::create_org_svc;
use crate::services::users::create_user_svc;
use crate::utils::{IdPrefix, generate_id};

const MIGRATIONS: &[&str] = &[
    include_str!("../db/migrations/02-create-users.sql"),
    include_str!("../db/migrations/03-create-passwords.sql"),
    include_str!("../db/migrations/04-create-orgs.sql"),
    include_str!("../db/migrations/05-create-org-members.sql"),
    include_str!("../db/migrations/06-create-apps.sql"),
    include_str!("../db/migrations/07-create-org-apps.sql"),
    include_str!("../db/migrations/08-create-oauth-codes.sql"),
    include_str!("../db/migrations/09-create-superusers.sql"),
];

pub struct TestCtx {
    pub state: AppState,
    pub db_dir: PathBuf,
}

pub struct AuthFixture {
    pub user: UserDto,
    pub org: OrgDto,
    pub email: String,
    pub password: String,
}

pub struct OauthFixture {
    pub auth: AuthFixture,
    pub app: AppDto,
}

impl AuthFixture {
    pub fn to_ctx(&self, scopes: Vec<Scope>) -> Ctx {
        let actor = Actor::new(
            ActorPayloadDto {
                id: self.user.id.clone(),
                org_id: self.org.id.clone(),
                org_count: 1,
                roles: vec![Role::OrgAdmin],
                scopes,
            },
            self.user.clone(),
        );

        Ctx::new(actor)
    }
}

impl Drop for TestCtx {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.db_dir);
    }
}

impl TestCtx {
    pub async fn new(test_name: &str) -> Result<Self> {
        let root = test_root_dir()?;
        let unique = generate_id(IdPrefix::User);
        let db_dir = root.join("test").join(format!("{}-{}", test_name, unique));
        let db_file = db_dir.join("yaas.db");

        fs::create_dir_all(&db_dir).context(IoSnafu)?;

        let conn = create_connection(&db_file).await?;
        run_migrations(&conn).await?;

        let mapper = create_db_mapper(db_file.as_path()).await?;

        let config = Config {
            server: ServerConfig {
                address: "127.0.0.1:0".to_string(),
                https: false,
            },
            db: DbConfig {
                dir: db_dir.clone(),
            },
            superuser: SuperuserConfig { setup_key: None },
            jwt_secret: "test-jwt-secret".to_string(),
            frontend_dir: db_dir.clone(),
            captcha_site_key: None,
            captcha_api_key: None,
            ga_tag_id: None,
            assets: AssetManifest {
                main_css: "".to_string(),
                main_js: "".to_string(),
            },
        };

        let client = ClientBuilder::new()
            .timeout(Duration::from_secs(3))
            .build()
            .expect("HTTP Client is required");

        let auth_cache = Cache::builder()
            .time_to_live(Duration::from_secs(10 * 60))
            .time_to_idle(Duration::from_secs(60))
            .max_capacity(100)
            .build();

        Ok(Self {
            state: AppState {
                config: Arc::new(config),
                db: Arc::new(mapper),
                client,
                auth_cache,
            },
            db_dir,
        })
    }

    pub async fn seed_user_with_password(
        &self,
        name: &str,
        email: &str,
        password: &str,
    ) -> Result<UserDto> {
        create_user_svc(
            &self.state,
            NewUserWithPasswordDto {
                name: name.to_string(),
                email: email.to_string(),
                password: password.to_string(),
            },
        )
        .await
    }

    pub async fn seed_auth_fixture(
        &self,
        name: &str,
        email: &str,
        password: &str,
        org_name: &str,
    ) -> Result<AuthFixture> {
        let user = self.seed_user_with_password(name, email, password).await?;

        let org = create_org_svc(
            &self.state,
            NewOrgDto {
                name: org_name.to_string(),
                owner_id: user.id.clone(),
            },
        )
        .await?;

        Ok(AuthFixture {
            user,
            org,
            email: email.to_string(),
            password: password.to_string(),
        })
    }

    pub async fn seed_app(&self, name: &str, redirect_uri: &str) -> Result<AppDto> {
        create_app_svc(
            &self.state,
            NewAppDto {
                name: name.to_string(),
                redirect_uri: redirect_uri.to_string(),
            },
        )
        .await
    }

    pub async fn seed_oauth_fixture(
        &self,
        name: &str,
        email: &str,
        password: &str,
        org_name: &str,
        app_name: &str,
        redirect_uri: &str,
        register_app: bool,
    ) -> Result<OauthFixture> {
        let auth = self
            .seed_auth_fixture(name, email, password, org_name)
            .await?;
        let app = self.seed_app(app_name, redirect_uri).await?;

        if register_app {
            create_org_app_svc(
                &self.state,
                &auth.org.id,
                NewOrgAppDto {
                    app_id: app.id.clone(),
                },
            )
            .await?;
        }

        Ok(OauthFixture { auth, app })
    }
}

fn test_root_dir() -> Result<PathBuf> {
    if let Ok(dir) = std::env::var("DATABASE_DIR")
        && !dir.is_empty()
    {
        return Ok(PathBuf::from(dir));
    }

    Ok(std::env::temp_dir().join("yaas"))
}

async fn create_connection(filename: &Path) -> Result<Connection> {
    let db = Builder::new_local(filename.to_str().expect("DB path is required"))
        .build()
        .await
        .context(DbBuilderSnafu)?;
    let conn = db.connect().context(DbConnectSnafu)?;

    Ok(conn)
}

async fn run_migrations(conn: &Connection) -> Result<()> {
    for migration in MIGRATIONS {
        for stmt in migration.split(';') {
            let sql = stmt.trim();
            if sql.is_empty() {
                continue;
            }

            let mut prepared = conn.prepare(sql).await.context(DbPrepareSnafu)?;
            prepared
                .execute(Vec::<(String, Value)>::new())
                .await
                .context(DbStatementSnafu)?;
        }
    }

    Ok(())
}
