use async_trait::async_trait;

use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;
use diesel::{AsChangeset, QueryDsl, SelectableHelper};
use serde::Deserialize;
use snafu::ResultExt;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu};
use crate::schema::users::{self, dsl};
use yaas::dto::UserDto;
use yaas::utils::generate_id;

const USER_ID_PREFIX: &'static str = "usr";

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

impl From<User> for UserDto {
    fn from(user: User) -> Self {
        UserDto {
            id: user.id,
            email: user.email,
            name: user.name,
            status: user.status,
            created_at: user.created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
            updated_at: user.created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewUser {
    pub email: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize, AsChangeset)]
#[diesel(table_name = crate::schema::users)]
pub struct UpdateUser {
    pub name: Option<String>,
    pub status: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait UserStore: Send + Sync {
    fn generate_id(&self) -> String;

    async fn list(&self) -> Result<Vec<UserDto>>;

    async fn create(&self, data: &NewUser) -> Result<UserDto>;

    async fn get(&self, id: &str) -> Result<Option<UserDto>>;

    async fn find_by_email(&self, email: &str) -> Result<Option<UserDto>>;

    async fn update(&self, id: &str, data: &UpdateUser) -> Result<bool>;

    async fn delete(&self, id: &str) -> Result<bool>;
}

pub struct UserRepo {
    db_pool: Pool,
}

impl UserRepo {
    pub fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl UserStore for UserRepo {
    fn generate_id(&self) -> String {
        generate_id(USER_ID_PREFIX)
    }

    async fn list(&self) -> Result<Vec<UserDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let select_res = db
            .interact(move |conn| {
                dsl::users
                    .filter(dsl::deleted_at.is_null())
                    .select(User::as_select())
                    .order(dsl::email.asc())
                    .load::<User>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let items = select_res.context(DbQuerySnafu {
            table: "users".to_string(),
        })?;

        let items: Vec<UserDto> = items.into_iter().map(|x| x.into()).collect();

        Ok(items)
    }

    async fn create(&self, data: &NewUser) -> Result<UserDto> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let data_copy = data.clone();
        let today = chrono::Utc::now();

        let user = User {
            id: generate_id(USER_ID_PREFIX),
            email: data_copy.email,
            name: data_copy.name,
            status: "active".to_string(),
            created_at: today.clone(),
            updated_at: today,
            deleted_at: None,
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

    async fn get(&self, id: &str) -> Result<Option<UserDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let id = id.to_string();
        let select_res = db
            .interact(move |conn| {
                dsl::users
                    .find(&id)
                    .filter(dsl::deleted_at.is_null())
                    .select(User::as_select())
                    .first::<User>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let user = select_res.context(DbQuerySnafu {
            table: "users".to_string(),
        })?;

        Ok(user.map(|x| x.into()))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<UserDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let email = email.to_string();
        let select_res = db
            .interact(move |conn| {
                dsl::users
                    .filter(dsl::email.eq(&email))
                    .filter(dsl::deleted_at.is_null())
                    .select(User::as_select())
                    .first::<User>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let user = select_res.context(DbQuerySnafu {
            table: "users".to_string(),
        })?;

        Ok(user.map(|x| x.into()))
    }

    async fn update(&self, id: &str, data: &UpdateUser) -> Result<bool> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let id = id.to_string();
        let mut data_clone = data.clone();
        if data_clone.updated_at.is_none() {
            data_clone.updated_at = Some(chrono::Utc::now());
        }
        let update_res = db
            .interact(move |conn| {
                diesel::update(dsl::users)
                    .filter(dsl::id.eq(&id))
                    .filter(dsl::deleted_at.is_null())
                    .set(data_clone)
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let affected = update_res.context(DbQuerySnafu {
            table: "users".to_string(),
        })?;

        Ok(affected > 0)
    }

    async fn delete(&self, id: &str) -> Result<bool> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        // Soft delete user
        let deleted_at = Some(chrono::Utc::now());

        let id = id.to_string();

        let update_res = db
            .interact(move |conn| {
                diesel::update(dsl::users)
                    .filter(dsl::id.eq(&id))
                    .filter(dsl::deleted_at.is_null())
                    .set(dsl::deleted_at.eq(deleted_at))
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let affected = update_res.context(DbQuerySnafu {
            table: "users".to_string(),
        })?;

        Ok(affected > 0)
    }
}

#[cfg(feature = "test")]
pub const TEST_USER_ID: &'static str = "usr_0196d1adc6807c2c8aa49982466faf88";

#[cfg(feature = "test")]
pub fn create_test_user() -> User {
    let today = chrono::Utc::now();

    User {
        id: TEST_USER_ID.to_string(),
        email: "user@example.com".to_string(),
        name: "user".to_string(),
        status: "active".to_string(),
        created_at: today.clone(),
        updated_at: today,
        deleted_at: None,
    }
}

#[cfg(feature = "test")]
pub struct UserTestRepo {}

#[cfg(feature = "test")]
#[async_trait]
impl UserStore for UserTestRepo {
    fn generate_id(&self) -> String {
        generate_id(USER_ID_PREFIX)
    }

    async fn list(&self) -> Result<Vec<UserDto>> {
        let user1 = create_test_user();
        let users = vec![user1];
        let filtered: Vec<UserDto> = users.into_iter().map(|x| x.into()).collect();
        Ok(filtered)
    }

    async fn create(&self, _data: &NewUser) -> Result<UserDto> {
        Err("Not supported".into())
    }

    async fn get(&self, id: &str) -> Result<Option<UserDto>> {
        let user1 = create_test_user();
        let users = vec![user1];
        let found = users.into_iter().find(|x| x.id.as_str() == id);
        Ok(found.map(|x| x.into()))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<UserDto>> {
        let user1 = create_test_user();
        let users = vec![user1];
        let found = users.into_iter().find(|x| x.email.as_str() == email);
        Ok(found.map(|x| x.into()))
    }

    async fn update(&self, _id: &str, _data: &UpdateUser) -> Result<bool> {
        Ok(true)
    }

    async fn delete(&self, _id: &str) -> Result<bool> {
        Ok(true)
    }
}
