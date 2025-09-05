use async_trait::async_trait;

use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;
use diesel::{QueryDsl, SelectableHelper};
use serde::Deserialize;
use snafu::ResultExt;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu};
use crate::schema::passwords::{self, dsl};
use yaas::dto::{NewPasswordDto, PasswordDto, UpdatePasswordDto};

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
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

#[derive(Debug, Clone, Deserialize, AsChangeset)]
#[diesel(table_name = crate::schema::passwords)]
struct UpdatePassword {
    password: Option<String>,
    updated_at: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait PasswordStore: Send + Sync {
    async fn create(&self, user_id: i32, data: NewPasswordDto) -> Result<()>;

    async fn get(&self, user_id: i32) -> Result<Option<PasswordDto>>;

    async fn update(&self, user_id: i32, data: UpdatePasswordDto) -> Result<bool>;

    async fn delete(&self, user_id: i32) -> Result<()>;
}

pub struct PasswordRepo {
    db_pool: Pool,
}

impl PasswordRepo {
    pub fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl PasswordStore for PasswordRepo {
    async fn create(&self, user_id: i32, data: NewPasswordDto) -> Result<()> {
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

    async fn get(&self, user_id: i32) -> Result<Option<PasswordDto>> {
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

    async fn update(&self, user_id: i32, data: UpdatePasswordDto) -> Result<bool> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let update_data = UpdatePassword {
            password: data.password.clone(),
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

    async fn delete(&self, user_id: i32) -> Result<()> {
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

#[cfg(feature = "test")]
pub fn create_test_password() -> Password {
    use crate::user::TEST_USER_ID;

    let today = chrono::Utc::now();

    Password {
        id: TEST_USER_ID,
        password: "password".to_string(),
        created_at: today.clone(),
        updated_at: today,
    }
}

#[cfg(feature = "test")]
pub struct PasswordTestRepo {}

#[cfg(feature = "test")]
#[async_trait]
impl PasswordStore for PasswordTestRepo {
    async fn create(&self, _user_id: i32, _data: NewPasswordDto) -> Result<()> {
        Err("Not supported".into())
    }

    async fn get(&self, id: i32) -> Result<Option<PasswordDto>> {
        let doc = create_test_password();
        let docs = vec![doc];
        let found = docs.into_iter().find(|x| x.id == id);
        Ok(found.map(|x| x.into()))
    }

    async fn update(&self, _user_id: i32, _data: UpdatePasswordDto) -> Result<bool> {
        Ok(true)
    }

    async fn delete(&self, _id: i32) -> Result<()> {
        Ok(())
    }
}
