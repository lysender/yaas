use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::dsl::count_star;
use diesel::prelude::*;
use diesel::{QueryDsl, SelectableHelper};
use snafu::ResultExt;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu};
use crate::schema::apps;
use crate::schema::org_apps::{self, dsl};
use yaas::dto::{ListOrgAppsParamsDto, NewOrgAppDto, OrgAppDto};
use yaas::pagination::{Paginated, PaginationParams};

#[derive(Clone, Queryable, Selectable)]
#[diesel(table_name = crate::schema::org_apps)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OrgApp {
    pub id: i32,
    pub org_id: i32,
    pub app_id: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Queryable)]
pub struct OrgAppWithName {
    pub id: i32,
    pub org_id: i32,
    pub app_id: i32,
    pub app_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl From<OrgAppWithName> for OrgAppDto {
    fn from(org: OrgAppWithName) -> Self {
        OrgAppDto {
            id: org.id,
            org_id: org.org_id,
            app_id: org.app_id,
            app_name: org.app_name,
            created_at: org.created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
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
            app_name: None,
            created_at: org.created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}

#[async_trait]
pub trait OrgAppStore: Send + Sync {
    async fn list(&self, org_id: i32, params: ListOrgAppsParamsDto)
    -> Result<Paginated<OrgAppDto>>;

    async fn create(&self, org_id: i32, data: NewOrgAppDto) -> Result<OrgAppDto>;

    async fn get(&self, id: i32) -> Result<Option<OrgAppDto>>;

    async fn find_app(&self, org_id: i32, app_id: i32) -> Result<Option<OrgAppDto>>;

    async fn delete(&self, id: i32) -> Result<()>;
}

pub struct OrgAppRepo {
    db_pool: Pool,
}

impl OrgAppRepo {
    pub fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }

    async fn listing_count(&self, org_id: i32, params: ListOrgAppsParamsDto) -> Result<i64> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let count_res = db
            .interact(move |conn| {
                let mut query = dsl::org_apps
                    .left_outer_join(apps::table.on(apps::id.eq(org_apps::app_id)))
                    .into_boxed();

                if let Some(keyword) = params.keyword {
                    if keyword.len() > 0 {
                        let pattern = format!("%{}%", keyword);
                        query = query.filter(apps::name.ilike(pattern));
                    }
                }

                query
                    .filter(dsl::org_id.eq(org_id))
                    .filter(apps::deleted_at.is_null())
                    .select(count_star())
                    .get_result::<i64>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let count = count_res.context(DbQuerySnafu {
            table: "org_apps".to_string(),
        })?;

        Ok(count)
    }
}

#[async_trait]
impl OrgAppStore for OrgAppRepo {
    async fn list(
        &self,
        org_id: i32,
        params: ListOrgAppsParamsDto,
    ) -> Result<Paginated<OrgAppDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let total_records = self.listing_count(org_id, params.clone()).await?;

        let pagination = PaginationParams::new(total_records, params.page, params.per_page, None);

        // Do not query if we already know there are no records
        if pagination.total_pages == 0 {
            return Ok(Paginated::new(
                Vec::new(),
                pagination.page,
                pagination.per_page,
                pagination.total_records,
            ));
        }

        let select_res = db
            .interact(move |conn| {
                let mut query = dsl::org_apps
                    .left_outer_join(apps::table.on(apps::id.eq(org_apps::app_id)))
                    .into_boxed();

                if let Some(keyword) = params.keyword {
                    if keyword.len() > 0 {
                        let pattern = format!("%{}%", keyword);
                        query = query.filter(apps::name.ilike(pattern));
                    }
                }

                query
                    .filter(dsl::org_id.eq(org_id))
                    .filter(apps::deleted_at.is_null())
                    .order_by(apps::name.asc())
                    .select((
                        org_apps::id,
                        org_apps::org_id,
                        org_apps::app_id,
                        apps::name.nullable(),
                        org_apps::created_at,
                    ))
                    .load::<OrgAppWithName>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let items = select_res.context(DbQuerySnafu {
            table: "org_apps".to_string(),
        })?;

        let items: Vec<OrgAppDto> = items.into_iter().map(|x| x.into()).collect();

        Ok(Paginated::new(
            items,
            pagination.page,
            pagination.per_page,
            pagination.total_records,
        ))
    }

    async fn create(&self, org_id: i32, data: NewOrgAppDto) -> Result<OrgAppDto> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let today = chrono::Utc::now();

        let new_doc = InsertableOrgApp {
            org_id,
            app_id: data.app_id,
            created_at: today,
        };

        let doc_copy = new_doc.clone();
        let inser_res = db
            .interact(move |conn| {
                diesel::insert_into(org_apps::table)
                    .values(doc_copy)
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

    async fn get(&self, id: i32) -> Result<Option<OrgAppDto>> {
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

        let org_app = select_res.context(DbQuerySnafu {
            table: "org_apps".to_string(),
        })?;

        Ok(org_app.map(|x| x.into()))
    }

    async fn find_app(&self, org_id: i32, app_id: i32) -> Result<Option<OrgAppDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let select_res = db
            .interact(move |conn| {
                dsl::org_apps
                    .filter(dsl::org_id.eq(org_id))
                    .filter(dsl::app_id.eq(app_id))
                    .select(OrgApp::as_select())
                    .first::<OrgApp>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let org_app = select_res.context(DbQuerySnafu {
            table: "org_apps".to_string(),
        })?;

        Ok(org_app.map(|x| x.into()))
    }

    async fn delete(&self, id: i32) -> Result<()> {
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

#[cfg(feature = "test")]
pub const TEST_ORG_APP_ID: i32 = 4000;

#[cfg(feature = "test")]
pub fn create_test_org_app() -> OrgApp {
    use crate::{app::TEST_APP_ID, org::TEST_ORG_ID};

    let today = chrono::Utc::now();

    OrgApp {
        id: TEST_ORG_APP_ID,
        org_id: TEST_ORG_ID,
        app_id: TEST_APP_ID,
        created_at: today,
    }
}

#[cfg(feature = "test")]
pub struct OrgAppTestRepo {}

#[cfg(feature = "test")]
#[async_trait]
impl OrgAppStore for OrgAppTestRepo {
    async fn list(
        &self,
        _org_id: i32,
        _params: ListOrgAppsParamsDto,
    ) -> Result<Paginated<OrgAppDto>> {
        let doc1 = create_test_org_app();
        let docs = vec![doc1];
        let total_records = docs.len() as i64;
        let filtered: Vec<OrgAppDto> = docs.into_iter().map(|x| x.into()).collect();
        Ok(Paginated::new(filtered, 1, 10, total_records))
    }

    async fn create(&self, _org_id: i32, _data: NewOrgAppDto) -> Result<OrgAppDto> {
        Err("Not supported".into())
    }

    async fn get(&self, id: i32) -> Result<Option<OrgAppDto>> {
        let doc1 = create_test_org_app();
        let docs = vec![doc1];
        let found = docs.into_iter().find(|x| x.id == id);
        Ok(found.map(|x| x.into()))
    }

    async fn find_app(&self, org_id: i32, app_id: i32) -> Result<Option<OrgAppDto>> {
        let doc1 = create_test_org_app();
        let docs = vec![doc1];
        let found = docs
            .into_iter()
            .find(|x| x.org_id == org_id && x.app_id == app_id);
        Ok(found.map(|x| x.into()))
    }

    async fn delete(&self, _id: i32) -> Result<()> {
        Ok(())
    }
}
