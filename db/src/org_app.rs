use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::QueryDsl;
use diesel::dsl::count_star;
use diesel::prelude::*;
use snafu::ResultExt;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu};
use crate::schema::apps;
use crate::schema::org_apps::{self, dsl};
use yaas::dto::{ListOrgAppsParamsDto, NewOrgAppDto, OrgAppDto, OrgAppSuggestionDto};
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

#[derive(Queryable)]
pub struct OrgAppSuggestion {
    pub id: i32,
    pub name: String,
}

impl From<OrgAppSuggestion> for OrgAppSuggestionDto {
    fn from(suggestion: OrgAppSuggestion) -> Self {
        OrgAppSuggestionDto {
            id: suggestion.id,
            name: suggestion.name,
        }
    }
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

    pub async fn list(
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
                    .limit(pagination.per_page as i64)
                    .offset(pagination.offset)
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

    async fn list_app_suggestions_count(
        &self,
        org_id: i32,
        params: ListOrgAppsParamsDto,
    ) -> Result<i64> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let count_res = db
            .interact(move |conn| {
                let mut query = apps::dsl::apps
                    .left_outer_join(
                        org_apps::table
                            .on(org_apps::app_id.eq(apps::id).and(dsl::org_id.eq(org_id))),
                    )
                    .into_boxed();

                if let Some(keyword) = params.keyword {
                    if keyword.len() > 0 {
                        let pattern = format!("%{}%", keyword);
                        query = query.filter(apps::name.ilike(pattern.clone()));
                    }
                }

                query
                    .filter(org_apps::app_id.is_null())
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

    pub async fn list_app_suggestions(
        &self,
        org_id: i32,
        params: ListOrgAppsParamsDto,
    ) -> Result<Paginated<OrgAppSuggestionDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let total_records = self
            .list_app_suggestions_count(org_id, params.clone())
            .await?;

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
                let mut query = apps::dsl::apps
                    .left_outer_join(
                        org_apps::table
                            .on(org_apps::app_id.eq(apps::id).and(dsl::org_id.eq(org_id))),
                    )
                    .into_boxed();

                if let Some(keyword) = params.keyword {
                    if keyword.len() > 0 {
                        let pattern = format!("%{}%", keyword);
                        query = query.filter(apps::name.ilike(pattern.clone()));
                    }
                }

                query
                    .filter(org_apps::app_id.is_null())
                    .filter(apps::deleted_at.is_null())
                    .order_by(apps::name.asc())
                    .limit(pagination.per_page as i64)
                    .offset(pagination.offset)
                    .select((apps::id, apps::name))
                    .load::<OrgAppSuggestion>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let items = select_res.context(DbQuerySnafu {
            table: "org_apps".to_string(),
        })?;

        let items: Vec<OrgAppSuggestionDto> = items.into_iter().map(|x| x.into()).collect();
        Ok(Paginated::new(
            items,
            pagination.page,
            pagination.per_page,
            pagination.total_records,
        ))
    }

    pub async fn create(&self, org_id: i32, data: NewOrgAppDto) -> Result<OrgAppDto> {
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

    pub async fn get(&self, id: i32) -> Result<Option<OrgAppDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let select_res = db
            .interact(move |conn| {
                dsl::org_apps
                    .left_outer_join(apps::table.on(apps::id.eq(org_apps::app_id)))
                    .filter(dsl::id.eq(id))
                    .filter(apps::deleted_at.is_null())
                    .select((
                        org_apps::id,
                        org_apps::org_id,
                        org_apps::app_id,
                        apps::name.nullable(),
                        org_apps::created_at,
                    ))
                    .first::<OrgAppWithName>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let org_app = select_res.context(DbQuerySnafu {
            table: "org_apps".to_string(),
        })?;

        Ok(org_app.map(|x| x.into()))
    }

    pub async fn find_app(&self, org_id: i32, app_id: i32) -> Result<Option<OrgAppDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let select_res = db
            .interact(move |conn| {
                dsl::org_apps
                    .left_outer_join(apps::table.on(apps::id.eq(org_apps::app_id)))
                    .filter(dsl::org_id.eq(org_id))
                    .filter(dsl::app_id.eq(app_id))
                    .filter(apps::deleted_at.is_null())
                    .select((
                        org_apps::id,
                        org_apps::org_id,
                        org_apps::app_id,
                        apps::name.nullable(),
                        org_apps::created_at,
                    ))
                    .first::<OrgAppWithName>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let org_app = select_res.context(DbQuerySnafu {
            table: "org_apps".to_string(),
        })?;

        Ok(org_app.map(|x| x.into()))
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
