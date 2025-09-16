use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::dsl::count_star;
use diesel::prelude::*;
use diesel::{QueryDsl, SelectableHelper};
use serde::Deserialize;
use snafu::ResultExt;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu};
use crate::schema::org_apps::{self, dsl};
use yaas::dto::OrgAppDto;

#[derive(Clone, Queryable, Selectable)]
#[diesel(table_name = crate::schema::org_apps)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OrgApp {
    pub id: i32,
    pub org_id: i32,
    pub app_id: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::org_apps)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct InsertableOrgApp {
    pub org_id: i32,
    pub app_id: i32,
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

#[derive(Clone, Deserialize)]
pub struct NewOrgApp {
    pub org_id: i32,
    pub app_id: i32,
}

pub struct OrgAppRepo {
    db_pool: Pool,
}

impl OrgAppRepo {
    pub fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }

    pub async fn list(&self) -> Result<Vec<OrgAppDto>> {
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

    pub async fn create(&self, data: &NewOrgApp) -> Result<OrgAppDto> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let data_copy = data.clone();
        let today = chrono::Utc::now();

        let new_doc = InsertableOrgApp {
            org_id: data_copy.org_id,
            app_id: data_copy.app_id,
            created_at: today,
        };

        let doc_copy = new_doc.clone();
        let inser_res = db
            .interact(move |conn| {
                diesel::insert_into(org_apps::table)
                    .values(&doc_copy)
                    .returning(org_apps::id)
                    .get_result(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let id: i32 = inser_res.context(DbQuerySnafu {
            table: "org_apps".to_string(),
        })?;

        let doc = OrgApp {
            id,
            org_id: new_doc.org_id,
            app_id: new_doc.app_id,
            created_at: new_doc.created_at,
        };

        Ok(doc.into())
    }

    pub async fn get(&self, id: i32) -> Result<Option<OrgAppDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let select_res = db
            .interact(move |conn| {
                dsl::org_apps
                    .find(id)
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

    pub async fn delete(&self, id: i32) -> Result<()> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let delete_res = db
            .interact(move |conn| {
                diesel::delete(dsl::org_apps.filter(dsl::id.eq(id))).execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let _ = delete_res.context(DbQuerySnafu {
            table: "org_apps".to_string(),
        })?;

        Ok(())
    }
}
