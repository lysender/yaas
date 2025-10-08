use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::dsl::count_star;
use diesel::prelude::*;
use diesel::{AsChangeset, QueryDsl, SelectableHelper};
use serde::Deserialize;
use snafu::ResultExt;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu};
use crate::schema::orgs::{self, dsl};
use crate::schema::users;
use yaas::dto::{ListOrgsParamsDto, NewOrgDto, OrgDto, UpdateOrgDto};
use yaas::pagination::{Paginated, PaginationParams};

#[derive(Clone, Queryable, Selectable)]
#[diesel(table_name = crate::schema::orgs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Org {
    pub id: i32,
    pub name: String,
    pub status: String,
    pub owner_id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Queryable)]
pub struct OrgWithOwner {
    pub id: i32,
    pub name: String,
    pub status: String,
    pub owner_id: i32,
    pub owner_email: Option<String>,
    pub owner_name: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::orgs)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct InsertableOrg {
    pub name: String,
    pub status: String,
    pub owner_id: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Org> for OrgDto {
    fn from(org: Org) -> Self {
        OrgDto {
            id: org.id,
            name: org.name,
            status: org.status,
            owner_id: org.owner_id,
            owner_email: None,
            owner_name: None,
            created_at: org.created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
            updated_at: org.created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}

impl From<OrgWithOwner> for OrgDto {
    fn from(org: OrgWithOwner) -> Self {
        OrgDto {
            id: org.id,
            name: org.name,
            status: org.status,
            owner_id: org.owner_id,
            owner_email: org.owner_email,
            owner_name: org.owner_name,
            created_at: org.created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
            updated_at: org.created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}

#[derive(Clone, Deserialize, AsChangeset)]
#[diesel(table_name = crate::schema::orgs)]
pub struct UpdateOrg {
    pub name: Option<String>,
    pub status: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

pub struct OrgRepo {
    db_pool: Pool,
}

impl OrgRepo {
    pub fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }

    async fn listing_count(&self, params: ListOrgsParamsDto) -> Result<i64> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let count_res = db
            .interact(move |conn| {
                let mut query = dsl::orgs
                    .left_outer_join(users::table.on(users::id.eq(orgs::owner_id)))
                    .into_boxed();
                query = query.filter(dsl::deleted_at.is_null());

                if let Some(keyword) = params.keyword {
                    if keyword.len() > 0 {
                        let pattern = format!("%{}%", keyword);
                        query = query.filter(
                            dsl::name
                                .ilike(pattern.clone())
                                .or(users::email.ilike(pattern)),
                        );
                    }
                }
                query.select(count_star()).get_result::<i64>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let count = count_res.context(DbQuerySnafu {
            table: "orgs".to_string(),
        })?;

        Ok(count)
    }

    pub async fn list(&self, params: ListOrgsParamsDto) -> Result<Paginated<OrgDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let total_records = self.listing_count(params.clone()).await?;

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
                let mut query = dsl::orgs
                    .left_outer_join(users::table.on(users::id.eq(orgs::owner_id)))
                    .into_boxed();
                query = query.filter(dsl::deleted_at.is_null());

                if let Some(keyword) = params.keyword {
                    if keyword.len() > 0 {
                        let pattern = format!("%{}%", keyword);
                        query = query.filter(
                            dsl::name
                                .ilike(pattern.clone())
                                .or(users::email.ilike(pattern)),
                        );
                    }
                }
                query
                    .limit(pagination.per_page as i64)
                    .offset(pagination.offset)
                    .select(Org::as_select())
                    .order_by(dsl::name.desc())
                    .select((
                        dsl::id,
                        dsl::name,
                        dsl::status,
                        dsl::owner_id,
                        users::name.nullable(),
                        users::email.nullable(),
                        dsl::created_at,
                        dsl::updated_at,
                    ))
                    .load::<OrgWithOwner>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let items = select_res.context(DbQuerySnafu {
            table: "orgs".to_string(),
        })?;

        let items: Vec<OrgDto> = items.into_iter().map(|x| x.into()).collect();

        Ok(Paginated::new(
            items,
            pagination.page,
            pagination.per_page,
            pagination.total_records,
        ))
    }

    pub async fn create(&self, data: NewOrgDto) -> Result<OrgDto> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let today = chrono::Utc::now();

        let new_org = InsertableOrg {
            name: data.name,
            status: "active".to_string(),
            owner_id: data.owner_id,
            created_at: today.clone(),
            updated_at: today,
        };

        let org_copy = new_org.clone();
        let inser_res = db
            .interact(move |conn| {
                diesel::insert_into(orgs::table)
                    .values(&org_copy)
                    .returning(orgs::id)
                    .get_result(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let id: i32 = inser_res.context(DbQuerySnafu {
            table: "orgs".to_string(),
        })?;

        let org = Org {
            id,
            name: new_org.name,
            status: new_org.status,
            owner_id: new_org.owner_id,
            created_at: new_org.created_at,
            updated_at: new_org.updated_at,
            deleted_at: None,
        };

        Ok(org.into())
    }

    pub async fn get(&self, id: i32) -> Result<Option<OrgDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let select_res = db
            .interact(move |conn| {
                dsl::orgs
                    .find(id)
                    .filter(dsl::deleted_at.is_null())
                    .select(Org::as_select())
                    .first::<Org>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let org = select_res.context(DbQuerySnafu {
            table: "orgs".to_string(),
        })?;

        Ok(org.map(|x| x.into()))
    }

    pub async fn update(&self, id: i32, data: UpdateOrgDto) -> Result<bool> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        // Do not allow empty update
        if data.status.is_none() && data.name.is_none() && data.owner_id.is_none() {
            return Ok(false);
        }

        let updated_org = UpdateOrg {
            name: data.name,
            status: data.status,
            updated_at: Some(chrono::Utc::now()),
        };

        let update_res = db
            .interact(move |conn| {
                diesel::update(dsl::orgs)
                    .filter(dsl::id.eq(id))
                    .filter(dsl::deleted_at.is_null())
                    .set(updated_org)
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let affected = update_res.context(DbQuerySnafu {
            table: "orgs".to_string(),
        })?;

        Ok(affected > 0)
    }

    pub async fn delete(&self, id: i32) -> Result<bool> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        // Soft delete by setting deleted_at to current time
        let deleted_at = Some(chrono::Utc::now());

        let update_res = db
            .interact(move |conn| {
                diesel::update(dsl::orgs)
                    .filter(dsl::id.eq(id))
                    .filter(dsl::deleted_at.is_null())
                    .set(dsl::deleted_at.eq(deleted_at))
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let affected = update_res.context(DbQuerySnafu {
            table: "orgs".to_string(),
        })?;

        Ok(affected > 0)
    }

    pub async fn test_read(&self) -> Result<()> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let selected_res = db
            .interact(move |conn| {
                dsl::orgs
                    .select(Org::as_select())
                    .first::<Org>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let _ = selected_res.context(DbQuerySnafu {
            table: "orgs".to_string(),
        })?;

        Ok(())
    }
}
