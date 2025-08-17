use async_trait::async_trait;

use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;
use diesel::{QueryDsl, SelectableHelper};
use serde::Deserialize;
use snafu::ResultExt;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu};
use crate::schema::org_apps::{self, dsl};
use yaas::dto::OrgAppDto;
use yaas::utils::generate_id;

const ORG_APP_ID_PREFIX: &'static str = "oap";

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::org_apps)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OrgApp {
    pub id: String,
    pub org_id: String,
    pub app_id: String,
    pub created_at: DateTime<Utc>,
}

impl From<OrgApp> for OrgAppDto {
    fn from(org: OrgApp) -> Self {
        OrgAppDto {
            id: org.id,
            org_id: org.org_id,
            app_id: org.app_id,
            created_at: org.created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewOrgApp {
    pub org_id: String,
    pub app_id: String,
}

#[async_trait]
pub trait OrgAppStore: Send + Sync {
    fn generate_id(&self) -> String;

    async fn list(&self) -> Result<Vec<OrgAppDto>>;

    async fn create(&self, data: &NewOrgApp) -> Result<OrgAppDto>;

    async fn get(&self, id: &str) -> Result<Option<OrgAppDto>>;

    async fn delete(&self, id: &str) -> Result<()>;
}

pub struct OrgAppRepo {
    db_pool: Pool,
}

impl OrgAppRepo {
    pub fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl OrgAppStore for OrgAppRepo {
    fn generate_id(&self) -> String {
        generate_id(ORG_APP_ID_PREFIX)
    }

    async fn list(&self) -> Result<Vec<OrgAppDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let select_res = db
            .interact(move |conn| {
                dsl::org_apps
                    .select(OrgApp::as_select())
                    .load::<OrgApp>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let items = select_res.context(DbQuerySnafu {
            table: "org_apps".to_string(),
        })?;

        let items: Vec<OrgAppDto> = items.into_iter().map(|x| x.into()).collect();

        Ok(items)
    }

    async fn create(&self, data: &NewOrgApp) -> Result<OrgAppDto> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let data_copy = data.clone();
        let today = chrono::Utc::now();

        let doc = OrgApp {
            id: generate_id(ORG_APP_ID_PREFIX),
            org_id: data_copy.org_id,
            app_id: data_copy.app_id,
            created_at: today,
        };

        let doc_copy = doc.clone();
        let inser_res = db
            .interact(move |conn| {
                diesel::insert_into(org_apps::table)
                    .values(&doc_copy)
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let _ = inser_res.context(DbQuerySnafu {
            table: "org_apps".to_string(),
        })?;

        Ok(doc.into())
    }

    async fn get(&self, id: &str) -> Result<Option<OrgAppDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let id = id.to_string();
        let select_res = db
            .interact(move |conn| {
                dsl::org_apps
                    .find(&id)
                    .select(OrgApp::as_select())
                    .first::<OrgApp>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let org = select_res.context(DbQuerySnafu {
            table: "org_apps".to_string(),
        })?;

        Ok(org.map(|x| x.into()))
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let id = id.to_string();
        let delete_res = db
            .interact(move |conn| {
                diesel::delete(dsl::org_apps.filter(dsl::id.eq(&id))).execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let _ = delete_res.context(DbQuerySnafu {
            table: "org_apps".to_string(),
        })?;

        Ok(())
    }
}

#[cfg(feature = "test")]
pub const TEST_ORG_APP_ID: &'static str = "oap_019896b7c4e97c3498b9bd9264266024";

#[cfg(feature = "test")]
pub fn create_test_org_app() -> OrgApp {
    use crate::{app::TEST_APP_ID, org::TEST_ORG_ID};

    let today = chrono::Utc::now();

    OrgApp {
        id: TEST_ORG_APP_ID.to_string(),
        org_id: TEST_ORG_ID.to_string(),
        app_id: TEST_APP_ID.to_string(),
        created_at: today,
    }
}

#[cfg(feature = "test")]
pub struct OrgAppTestRepo {}

#[cfg(feature = "test")]
#[async_trait]
impl OrgAppStore for OrgAppTestRepo {
    fn generate_id(&self) -> String {
        generate_id(ORG_APP_ID_PREFIX)
    }

    async fn list(&self) -> Result<Vec<OrgAppDto>> {
        let doc1 = create_test_org_app();
        let docs = vec![doc1];
        let filtered: Vec<OrgAppDto> = docs.into_iter().map(|x| x.into()).collect();
        Ok(filtered)
    }

    async fn create(&self, _data: &NewOrgApp) -> Result<OrgAppDto> {
        Err("Not supported".into())
    }

    async fn get(&self, id: &str) -> Result<Option<OrgAppDto>> {
        let doc1 = create_test_org_app();
        let docs = vec![doc1];
        let found = docs.into_iter().find(|x| x.id.as_str() == id);
        Ok(found.map(|x| x.into()))
    }

    async fn delete(&self, _id: &str) -> Result<()> {
        Ok(())
    }
}
