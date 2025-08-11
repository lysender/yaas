use async_trait::async_trait;

use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;
use diesel::{AsChangeset, QueryDsl, SelectableHelper};
use serde::Deserialize;
use snafu::ResultExt;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu};
use crate::schema::apps::{self, dsl};
use yaas::dto::AppDto;
use yaas::utils::generate_id;

const ORG_ID_PREFIX: &'static str = "app";

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::apps)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct App {
    pub id: String,
    pub name: String,
    pub status: String,
    pub owner_id: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<App> for AppDto {
    fn from(app: App) -> Self {
        AppDto {
            id: app.id,
            name: app.name,
            status: app.status,
            owner_id: app.owner_id,
            created_at: app.created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
            updated_at: app.created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewApp {
    pub name: String,
    pub owner_id: String,
}

#[derive(Debug, Clone, Deserialize, AsChangeset)]
#[diesel(table_name = crate::schema::apps)]
pub struct UpdateApp {
    pub name: Option<String>,
    pub status: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait AppStore: Send + Sync {
    async fn list(&self) -> Result<Vec<AppDto>>;

    async fn create(&self, data: &NewApp) -> Result<AppDto>;

    async fn get(&self, id: &str) -> Result<Option<AppDto>>;

    async fn update(&self, id: &str, data: &UpdateApp) -> Result<bool>;

    async fn delete(&self, id: &str) -> Result<()>;
}

pub struct AppRepo {
    db_pool: Pool,
}

impl AppRepo {
    pub fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl AppStore for AppRepo {
    async fn list(&self) -> Result<Vec<AppDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let select_res = db
            .interact(move |conn| {
                dsl::apps
                    .select(App::as_select())
                    .order(dsl::name.asc())
                    .load::<App>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let items = select_res.context(DbQuerySnafu {
            table: "apps".to_string(),
        })?;

        let items: Vec<AppDto> = items.into_iter().map(|x| x.into()).collect();

        Ok(items)
    }

    async fn create(&self, data: &NewApp) -> Result<AppDto> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let data_copy = data.clone();
        let today = chrono::Utc::now();

        let doc = App {
            id: generate_id(ORG_ID_PREFIX),
            name: data_copy.name,
            status: "active".to_string(),
            owner_id: data_copy.owner_id,
            created_at: today.clone(),
            updated_at: today,
        };

        let doc_copy = doc.clone();
        let inser_res = db
            .interact(move |conn| {
                diesel::insert_into(apps::table)
                    .values(&doc_copy)
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let _ = inser_res.context(DbQuerySnafu {
            table: "apps".to_string(),
        })?;

        Ok(doc.into())
    }

    async fn get(&self, id: &str) -> Result<Option<AppDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let id = id.to_string();
        let select_res = db
            .interact(move |conn| {
                dsl::apps
                    .find(&id)
                    .select(App::as_select())
                    .first::<App>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let app = select_res.context(DbQuerySnafu {
            table: "apps".to_string(),
        })?;

        Ok(app.map(|x| x.into()))
    }

    async fn update(&self, id: &str, data: &UpdateApp) -> Result<bool> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let id = id.to_string();
        let mut data_clone = data.clone();
        if data_clone.updated_at.is_none() {
            data_clone.updated_at = Some(chrono::Utc::now());
        }
        let update_res = db
            .interact(move |conn| {
                diesel::update(dsl::apps)
                    .filter(dsl::id.eq(&id))
                    .set(data_clone)
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let affected = update_res.context(DbQuerySnafu {
            table: "apps".to_string(),
        })?;

        Ok(affected > 0)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let id = id.to_string();
        let delete_res = db
            .interact(move |conn| diesel::delete(dsl::apps.filter(dsl::id.eq(&id))).execute(conn))
            .await
            .context(DbInteractSnafu)?;

        let _ = delete_res.context(DbQuerySnafu {
            table: "apps".to_string(),
        })?;

        Ok(())
    }
}

#[cfg(feature = "test")]
pub const TEST_ORG_ID: &'static str = "app_019896b7c4e97c3498b9bd9264266024";

#[cfg(feature = "test")]
pub fn create_test_app() -> Result<App> {
    use crate::user::TEST_USER_ID;

    let today = chrono::Utc::now();

    Ok(App {
        id: TEST_ORG_ID.to_string(),
        name: "app".to_string(),
        status: "active".to_string(),
        owner_id: TEST_USER_ID.to_string(),
        created_at: today.clone(),
        updated_at: today,
    })
}

#[cfg(feature = "test")]
pub struct AppTestRepo {}

#[cfg(feature = "test")]
#[async_trait]
impl AppStore for AppTestRepo {
    async fn list(&self) -> Result<Vec<AppDto>> {
        let app1 = create_test_app()?;
        let apps = vec![app1];
        let filtered: Vec<AppDto> = apps.into_iter().map(|x| x.into()).collect();
        Ok(filtered)
    }

    async fn create(&self, _data: &NewApp) -> Result<AppDto> {
        Err("Not supported".into())
    }

    async fn get(&self, id: &str) -> Result<Option<AppDto>> {
        let app1 = create_test_app()?;
        let apps = vec![app1];
        let found = apps.into_iter().find(|x| x.id.as_str() == id);
        Ok(found.map(|x| x.into()))
    }

    async fn update(&self, _id: &str, _data: &UpdateApp) -> Result<bool> {
        Ok(true)
    }

    async fn delete(&self, _id: &str) -> Result<()> {
        Ok(())
    }
}
