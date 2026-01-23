use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::dsl::count_star;
use diesel::prelude::*;
use diesel::{AsChangeset, QueryDsl, SelectableHelper};
use serde::Deserialize;
use snafu::ResultExt;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu};
use crate::schema::apps::{self, dsl};
use yaas::dto::{AppDto, ListAppsParamsDto, NewAppDto, UpdateAppDto};
use yaas::pagination::{Paginated, PaginationParams};
use yaas::utils::generate_id;

#[derive(Clone, Queryable, Selectable)]
#[diesel(table_name = crate::schema::apps)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct App {
    pub id: i32,
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::apps)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct InsertableApp {
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<App> for AppDto {
    fn from(app: App) -> Self {
        AppDto {
            id: app.id,
            name: app.name,
            client_id: app.client_id,
            client_secret: app.client_secret,
            redirect_uri: app.redirect_uri,
            created_at: app.created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
            updated_at: app.created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}

#[derive(Clone, Deserialize, AsChangeset)]
#[diesel(table_name = crate::schema::apps)]
pub struct UpdateApp {
    pub name: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub redirect_uri: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

pub struct AppRepo {
    db_pool: Pool,
}

impl AppRepo {
    pub fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }

    async fn listing_count(&self, params: ListAppsParamsDto) -> Result<i64> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let count_res = db
            .interact(move |conn| {
                let mut query = dsl::apps.into_boxed();
                query = query.filter(dsl::deleted_at.is_null());

                if let Some(keyword) = params.keyword
                    && !keyword.is_empty()
                {
                    let pattern = format!("%{}%", keyword);
                    query = query.filter(dsl::name.ilike(pattern));
                }
                query.select(count_star()).get_result::<i64>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let count = count_res.context(DbQuerySnafu {
            table: "users".to_string(),
        })?;

        Ok(count)
    }

    pub async fn list(&self, params: ListAppsParamsDto) -> Result<Paginated<AppDto>> {
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
                let mut query = dsl::apps.into_boxed();
                query = query.filter(dsl::deleted_at.is_null());

                if let Some(keyword) = params.keyword
                    && !keyword.is_empty()
                {
                    let pattern = format!("%{}%", keyword);
                    query = query.filter(dsl::name.ilike(pattern));
                }
                query
                    .limit(pagination.per_page as i64)
                    .offset(pagination.offset)
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

        Ok(Paginated::new(
            items,
            pagination.page,
            pagination.per_page,
            pagination.total_records,
        ))
    }

    pub async fn create(&self, data: NewAppDto) -> Result<AppDto> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let today = chrono::Utc::now();

        let new_app = InsertableApp {
            name: data.name,
            client_id: generate_id("cli"),
            client_secret: generate_id("sec"),
            redirect_uri: data.redirect_uri,
            created_at: today,
            updated_at: today,
        };

        let doc_copy = new_app.clone();
        let insert_res = db
            .interact(move |conn| {
                diesel::insert_into(apps::table)
                    .values(&doc_copy)
                    .returning(apps::id)
                    .get_result(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let id: i32 = insert_res.context(DbQuerySnafu {
            table: "apps".to_string(),
        })?;

        let app = App {
            id,
            name: new_app.name,
            client_id: new_app.client_id,
            client_secret: new_app.client_secret,
            redirect_uri: new_app.redirect_uri,
            created_at: new_app.created_at,
            updated_at: new_app.updated_at,
            deleted_at: None,
        };

        Ok(app.into())
    }

    pub async fn get(&self, id: i32) -> Result<Option<AppDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let select_res = db
            .interact(move |conn| {
                dsl::apps
                    .find(id)
                    .filter(dsl::deleted_at.is_null())
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

    pub async fn update(&self, id: i32, data: UpdateAppDto) -> Result<bool> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        // Do not allow empty update
        if data.name.is_none() && data.redirect_uri.is_none() {
            return Ok(false);
        }

        let update_app = UpdateApp {
            name: data.name,
            client_id: None,
            client_secret: None,
            redirect_uri: data.redirect_uri,
            updated_at: Some(chrono::Utc::now()),
        };

        let update_res = db
            .interact(move |conn| {
                diesel::update(dsl::apps)
                    .filter(dsl::id.eq(id))
                    .filter(dsl::deleted_at.is_null())
                    .set(update_app)
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let affected = update_res.context(DbQuerySnafu {
            table: "apps".to_string(),
        })?;

        Ok(affected > 0)
    }

    pub async fn regenerate_secret(&self, id: i32) -> Result<bool> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let update_app = UpdateApp {
            name: None,
            client_id: Some(generate_id("cli")),
            client_secret: Some(generate_id("sec")),
            redirect_uri: None,
            updated_at: Some(chrono::Utc::now()),
        };

        let update_res = db
            .interact(move |conn| {
                diesel::update(dsl::apps)
                    .filter(dsl::id.eq(id))
                    .filter(dsl::deleted_at.is_null())
                    .set(update_app)
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let affected = update_res.context(DbQuerySnafu {
            table: "apps".to_string(),
        })?;

        Ok(affected > 0)
    }

    pub async fn delete(&self, id: i32) -> Result<bool> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let deleted_at = Some(chrono::Utc::now());

        let update_res = db
            .interact(move |conn| {
                diesel::update(dsl::apps)
                    .filter(dsl::id.eq(id))
                    .filter(dsl::deleted_at.is_null())
                    .set(dsl::deleted_at.eq(deleted_at))
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let affected = update_res.context(DbQuerySnafu {
            table: "apps".to_string(),
        })?;

        Ok(affected > 0)
    }
}
