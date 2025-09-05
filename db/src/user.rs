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
use yaas::dto::{NewUserDto, UpdateUserDto, UserDto};

#[derive(Debug, Clone, Queryable, Selectable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub id: i32,
    pub email: String,
    pub name: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Insertable)]
#[diesel(table_name = crate::schema::users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct InsertableUser {
    email: String,
    name: String,
    status: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
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

#[derive(Debug, Clone, Deserialize, AsChangeset)]
#[diesel(table_name = crate::schema::users)]
struct UpdateUser {
    name: Option<String>,
    status: Option<String>,
    updated_at: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait UserStore: Send + Sync {
    async fn list(&self) -> Result<Vec<UserDto>>;

    async fn create(&self, data: NewUserDto) -> Result<UserDto>;

    async fn get(&self, id: i32) -> Result<Option<UserDto>>;

    async fn find_by_email(&self, email: &str) -> Result<Option<UserDto>>;

    async fn update(&self, id: i32, data: UpdateUserDto) -> Result<bool>;

    async fn delete(&self, id: i32) -> Result<bool>;
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

    async fn create(&self, data: NewUserDto) -> Result<UserDto> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let today = chrono::Utc::now();

        let new_user = InsertableUser {
            email: data.email,
            name: data.name,
            status: "active".to_string(),
            created_at: today.clone(),
            updated_at: today,
        };

        let user_copy = new_user.clone();
        let inser_res = db
            .interact(move |conn| {
                diesel::insert_into(users::table)
                    .values(&user_copy)
                    .returning(users::id)
                    .get_result(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let id: i32 = inser_res.context(DbQuerySnafu {
            table: "users".to_string(),
        })?;

        let user = User {
            id,
            email: new_user.email,
            name: new_user.name,
            status: new_user.status,
            created_at: new_user.created_at,
            updated_at: new_user.updated_at,
            deleted_at: None,
        };

        Ok(user.into())
    }

    async fn get(&self, id: i32) -> Result<Option<UserDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let select_res = db
            .interact(move |conn| {
                dsl::users
                    .find(id)
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

    async fn update(&self, id: i32, data: UpdateUserDto) -> Result<bool> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let updated_user = UpdateUser {
            name: data.name,
            status: data.status,
            updated_at: Some(chrono::Utc::now()),
        };

        let update_res = db
            .interact(move |conn| {
                diesel::update(dsl::users)
                    .filter(dsl::id.eq(id))
                    .filter(dsl::deleted_at.is_null())
                    .set(updated_user)
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let affected = update_res.context(DbQuerySnafu {
            table: "users".to_string(),
        })?;

        Ok(affected > 0)
    }

    async fn delete(&self, id: i32) -> Result<bool> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        // Soft delete user
        let deleted_at = Some(chrono::Utc::now());

        let update_res = db
            .interact(move |conn| {
                diesel::update(dsl::users)
                    .filter(dsl::id.eq(id))
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
pub const TEST_USER_ID: i32 = 1000;

#[cfg(feature = "test")]
pub fn create_test_user() -> User {
    let today = chrono::Utc::now();

    User {
        id: TEST_USER_ID,
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
    async fn list(&self) -> Result<Vec<UserDto>> {
        let user1 = create_test_user();
        let users = vec![user1];
        let filtered: Vec<UserDto> = users.into_iter().map(|x| x.into()).collect();
        Ok(filtered)
    }

    async fn create(&self, _data: NewUserDto) -> Result<UserDto> {
        Err("Not supported".into())
    }

    async fn get(&self, id: i32) -> Result<Option<UserDto>> {
        let user1 = create_test_user();
        let users = vec![user1];
        let found = users.into_iter().find(|x| x.id == id);
        Ok(found.map(|x| x.into()))
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<UserDto>> {
        let user1 = create_test_user();
        let users = vec![user1];
        let found = users.into_iter().find(|x| x.email.as_str() == email);
        Ok(found.map(|x| x.into()))
    }

    async fn update(&self, _id: i32, _data: UpdateUserDto) -> Result<bool> {
        Ok(true)
    }

    async fn delete(&self, _id: i32) -> Result<bool> {
        Ok(true)
    }
}
