use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use moka::sync::Cache;
use reqwest::ClientBuilder;
use snafu::ResultExt;
use turso::{Builder, Connection, Value};

use crate::config::{AssetManifest, Config, DbConfig, ServerConfig, SuperuserConfig};
use crate::db::create_db_mapper;
use crate::dto::{NewOrgDto, NewUserWithPasswordDto, OrgDto, UserDto};
use crate::error::{DbBuilderSnafu, DbConnectSnafu, DbPrepareSnafu, DbStatementSnafu, IoSnafu};
use crate::run::AppState;
use crate::services::orgs::create_org_svc;
use crate::services::users::create_user_svc;
use crate::utils::{IdPrefix, generate_id};
use crate::{Error, Result};

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
}

fn test_root_dir() -> Result<PathBuf> {
    let Ok(dir) = std::env::var("DATABASE_DIR") else {
        return Err(Error::Config {
            msg: "DATABASE_DIR is required".into(),
        });
    };

    if dir.is_empty() {
        return Err(Error::Config {
            msg: "DATABASE_DIR is required".into(),
        });
    }

    Ok(PathBuf::from(dir))
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
