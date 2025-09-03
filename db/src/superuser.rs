use async_trait::async_trait;

use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;
use diesel::{QueryDsl, SelectableHelper};
use snafu::ResultExt;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu};
use crate::schema::superusers::{self, dsl};
use yaas::xdto::SuperuserDto;

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::superusers)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Superuser {
    pub id: i32,
    pub created_at: DateTime<Utc>,
}

impl From<Superuser> for SuperuserDto {
    fn from(user: Superuser) -> Self {
        SuperuserDto {
            id: user.id,
            created_at: user.created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}

#[async_trait]
pub trait SuperuserStore: Send + Sync {
    async fn list(&self) -> Result<Vec<SuperuserDto>>;

    async fn create(&self, user_id: i32) -> Result<SuperuserDto>;

    async fn get(&self, user_id: i32) -> Result<Option<SuperuserDto>>;
}

pub struct SuperuserRepo {
    db_pool: Pool,
}

impl SuperuserRepo {
    pub fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl SuperuserStore for SuperuserRepo {
    async fn list(&self) -> Result<Vec<SuperuserDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let select_res = db
            .interact(move |conn| {
                dsl::superusers
                    .select(Superuser::as_select())
                    .load::<Superuser>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let items = select_res.context(DbQuerySnafu {
            table: "superusers".to_string(),
        })?;

        let items: Vec<SuperuserDto> = items.into_iter().map(|x| x.into()).collect();

        Ok(items)
    }

    async fn create(&self, user_id: i32) -> Result<SuperuserDto> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let doc = Superuser {
            id: user_id,
            created_at: Utc::now(),
        };

        let doc_copy = doc.clone();

        let inser_res = db
            .interact(move |conn| {
                diesel::insert_into(superusers::table)
                    .values(&doc_copy)
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let _ = inser_res.context(DbQuerySnafu {
            table: "superusers".to_string(),
        })?;

        Ok(doc.into())
    }

    async fn get(&self, id: i32) -> Result<Option<SuperuserDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let select_res = db
            .interact(move |conn| {
                dsl::superusers
                    .find(id)
                    .select(Superuser::as_select())
                    .first::<Superuser>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let user = select_res.context(DbQuerySnafu {
            table: "superusers".to_string(),
        })?;

        Ok(user.map(|x| x.into()))
    }
}

#[cfg(feature = "test")]
pub struct SuperuserTestRepo {}

#[cfg(feature = "test")]
#[async_trait]
impl SuperuserStore for SuperuserTestRepo {
    async fn list(&self) -> Result<Vec<SuperuserDto>> {
        use crate::user::create_test_user;

        let user1 = create_test_user();
        let doc1 = Superuser {
            id: user1.id,
            created_at: user1.created_at,
        };
        let docs = vec![doc1];
        let filtered: Vec<SuperuserDto> = docs.into_iter().map(|x| x.into()).collect();
        Ok(filtered)
    }

    async fn create(&self, _user_id: i32) -> Result<SuperuserDto> {
        Err("Not supported".into())
    }

    async fn get(&self, id: i32) -> Result<Option<SuperuserDto>> {
        use crate::user::create_test_user;
        let user1 = create_test_user();
        let doc1 = Superuser {
            id: user1.id,
            created_at: user1.created_at,
        };
        let docs = vec![doc1];
        let found = docs.into_iter().find(|x| x.id == id);
        Ok(found.map(|x| x.into()))
    }
}
