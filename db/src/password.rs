use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;
use diesel::{QueryDsl, SelectableHelper};
use serde::Deserialize;
use snafu::ResultExt;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu};
use crate::schema::passwords::{self, dsl};
use yaas::dto::{NewPasswordDto, PasswordDto};

#[derive(Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::passwords)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Password {
    pub id: i32,
    pub password: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Password> for PasswordDto {
    fn from(password: Password) -> Self {
        PasswordDto {
            id: password.id,
            password: password.password,
            created_at: password
                .created_at
                .to_rfc3339_opts(SecondsFormat::Millis, true),
            updated_at: password
                .created_at
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}

#[derive(Clone, Deserialize, AsChangeset)]
#[diesel(table_name = crate::schema::passwords)]
struct UpdatePassword {
    password: Option<String>,
    updated_at: Option<DateTime<Utc>>,
}

pub struct PasswordRepo {
    db_pool: Pool,
}

impl PasswordRepo {
    pub fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }

    pub async fn create(&self, user_id: i32, data: NewPasswordDto) -> Result<()> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let today = chrono::Utc::now();

        let doc = Password {
            id: user_id,
            password: data.password,
            created_at: today.clone(),
            updated_at: today,
        };

        let inser_res = db
            .interact(move |conn| {
                diesel::insert_into(passwords::table)
                    .values(&doc)
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let _ = inser_res.context(DbQuerySnafu {
            table: "passwords".to_string(),
        })?;

        Ok(())
    }

    pub async fn get(&self, user_id: i32) -> Result<Option<PasswordDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let select_res = db
            .interact(move |conn| {
                dsl::passwords
                    .find(user_id)
                    .select(Password::as_select())
                    .first::<Password>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let doc = select_res.context(DbQuerySnafu {
            table: "passwords".to_string(),
        })?;

        Ok(doc.map(|x| x.into()))
    }

    pub async fn update(&self, user_id: i32, data: NewPasswordDto) -> Result<bool> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let update_data = UpdatePassword {
            password: Some(data.password.clone()),
            updated_at: Some(chrono::Utc::now()),
        };
        let update_res = db
            .interact(move |conn| {
                diesel::update(dsl::passwords)
                    .filter(dsl::id.eq(user_id))
                    .set(&update_data)
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let affected = update_res.context(DbQuerySnafu {
            table: "passwords".to_string(),
        })?;

        Ok(affected > 0)
    }

    pub async fn delete(&self, user_id: i32) -> Result<()> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let delete_res = db
            .interact(move |conn| {
                diesel::delete(dsl::passwords.filter(dsl::id.eq(user_id))).execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let _ = delete_res.context(DbQuerySnafu {
            table: "passwords".to_string(),
        })?;

        Ok(())
    }
}
