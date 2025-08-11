use async_trait::async_trait;

use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;
use diesel::{QueryDsl, SelectableHelper};
use serde::Deserialize;
use snafu::ResultExt;
use validator::Validate;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu};
use crate::schema::users::{self, dsl};
use yaas::dto::PasswordDto;
use yaas::utils::generate_id;

const PASSWORD_ID_PREFIX: &'static str = "pwd";

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::passwords)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Password {
    pub id: String,
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

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct NewPassword {
    #[validate(length(min = 8, max = 60))]
    pub password: String,
}

#[derive(Debug, Clone, Deserialize, Validate)]
pub struct UpdatePassword {
    #[validate(length(min = 8, max = 60))]
    pub password: String,
}

#[async_trait]
pub trait PasswordStore: Send + Sync {
    async fn create(&self, user_id: &str, data: &NewPassword) -> Result<PasswordDto>;

    async fn get(&self, user_id: &str) -> Result<Option<PasswordDto>>;

    async fn update(&self, user_id: &str, data: &UpdatePassword) -> Result<bool>;

    async fn delete(&self, user_id: &str) -> Result<()>;
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
    async fn create(&self, user_id: &str, data: &NewPassword) -> Result<PasswordDto> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let data_copy = data.clone();
        let today = chrono::Utc::now();

        let user = Password {
            id: generate_id(PASSWORD_ID_PREFIX),
            name: data_copy.name,
            status: "active".to_string(),
            created_at: today.clone(),
            updated_at: today,
        };

        let user_copy = user.clone();
        let inser_res = db
            .interact(move |conn| {
                diesel::insert_into(users::table)
                    .values(&user_copy)
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let _ = inser_res.context(DbQuerySnafu {
            table: "users".to_string(),
        })?;

        Ok(user.into())
    }

    async fn get(&self, id: &str) -> Result<Option<PasswordDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let id = id.to_string();
        let select_res = db
            .interact(move |conn| {
                dsl::users
                    .find(&id)
                    .select(Password::as_select())
                    .first::<Password>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let user = select_res.context(DbQuerySnafu {
            table: "users".to_string(),
        })?;

        Ok(user.map(|x| x.into()))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<PasswordDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let email = email.to_string();
        let select_res = db
            .interact(move |conn| {
                dsl::users
                    .filter(dsl::email.eq(&email))
                    .select(Password::as_select())
                    .first::<Password>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let user = select_res.context(DbQuerySnafu {
            table: "users".to_string(),
        })?;

        Ok(user.map(|x| x.into()))
    }

    async fn update_status(&self, id: &str, data: &UpdatePassword) -> Result<bool> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let id = id.to_string();
        let status = data.status.clone();
        let today = chrono::Utc::now();
        let update_res = db
            .interact(move |conn| {
                diesel::update(dsl::users)
                    .filter(dsl::id.eq(&id))
                    .set((dsl::status.eq(&status), dsl::updated_at.eq(today)))
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let affected = update_res.context(DbQuerySnafu {
            table: "users".to_string(),
        })?;

        Ok(affected > 0)
    }

    async fn delete(&self, id: &str) -> Result<()> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        // Make sure the user has no associations to other entities
        let id = id.to_string();
        let delete_res = db
            .interact(move |conn| diesel::delete(dsl::users.filter(dsl::id.eq(&id))).execute(conn))
            .await
            .context(DbInteractSnafu)?;

        let _ = delete_res.context(DbQuerySnafu {
            table: "users".to_string(),
        })?;

        Ok(())
    }
}

#[cfg(feature = "test")]
pub const TEST_USER_ID: &'static str = "0196d1adc6807c2c8aa49982466faf88";

#[cfg(feature = "test")]
pub fn create_test_user() -> Result<Password> {
    let today = chrono::Utc::now();

    Ok(Password {
        id: TEST_USER_ID.to_string(),
        email: "user@example.com".to_string(),
        name: "user".to_string(),
        status: "active".to_string(),
        created_at: today.clone(),
        updated_at: today,
    })
}

#[cfg(feature = "test")]
pub struct PasswordTestRepo {}

#[cfg(feature = "test")]
#[async_trait]
impl PasswordStore for PasswordTestRepo {
    async fn list(&self) -> Result<Vec<PasswordDto>> {
        let user1 = create_test_user()?;
        let users = vec![user1];
        let filtered: Vec<PasswordDto> = users.into_iter().map(|x| x.into()).collect();
        Ok(filtered)
    }

    async fn create(&self, _data: &NewPassword) -> Result<PasswordDto> {
        Err("Not supported".into())
    }

    async fn get(&self, id: &str) -> Result<Option<PasswordDto>> {
        let user1 = create_test_user()?;
        let users = vec![user1];
        let found = users.into_iter().find(|x| x.id.as_str() == id);
        Ok(found.map(|x| x.into()))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<PasswordDto>> {
        let user1 = create_test_user()?;
        let users = vec![user1];
        let found = users.into_iter().find(|x| x.email.as_str() == email);
        Ok(found.map(|x| x.into()))
    }

    async fn update_status(&self, _id: &str, _data: &UpdatePassword) -> Result<bool> {
        Ok(true)
    }

    async fn delete(&self, _id: &str) -> Result<()> {
        Ok(())
    }
}
