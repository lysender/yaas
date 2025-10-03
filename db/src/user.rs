use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::dsl::count_star;
use diesel::prelude::*;
use diesel::result::Error;
use diesel::{AsChangeset, QueryDsl, SelectableHelper};
use serde::Deserialize;
use snafu::ResultExt;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu};
use crate::schema::passwords;
use crate::schema::users::{self, dsl};
use yaas::dto::{ListUsersParamsDto, NewUserDto, NewUserWithPasswordDto, UpdateUserDto, UserDto};
use yaas::pagination::{Paginated, PaginationParams};

#[derive(Clone, Queryable, Selectable)]
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

#[derive(Clone, Insertable)]
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

#[derive(Clone, Deserialize, AsChangeset)]
#[diesel(table_name = crate::schema::users)]
struct UpdateUser {
    name: Option<String>,
    status: Option<String>,
    updated_at: Option<DateTime<Utc>>,
}

pub struct UserRepo {
    db_pool: Pool,
}

impl UserRepo {
    pub fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }

    async fn listing_count(&self, params: ListUsersParamsDto) -> Result<i64> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let count_res = db
            .interact(move |conn| {
                let mut query = dsl::users.into_boxed();
                query = query.filter(dsl::deleted_at.is_null());

                if let Some(keyword) = params.keyword {
                    if keyword.len() > 0 {
                        let pattern = format!("%{}%", keyword);
                        query = query.filter(
                            dsl::email
                                .ilike(pattern.clone())
                                .or(dsl::name.ilike(pattern)),
                        );
                    }
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

    pub async fn list(&self, params: ListUsersParamsDto) -> Result<Paginated<UserDto>> {
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
                let mut query = dsl::users.into_boxed();
                query = query.filter(dsl::deleted_at.is_null());

                if let Some(keyword) = params.keyword {
                    if keyword.len() > 0 {
                        let pattern = format!("%{}%", keyword);
                        query = query.filter(
                            dsl::email
                                .ilike(pattern.clone())
                                .or(dsl::name.ilike(pattern)),
                        );
                    }
                }
                query
                    .limit(pagination.per_page as i64)
                    .offset(pagination.offset)
                    .select(User::as_select())
                    .order(dsl::id.desc())
                    .load::<User>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let items = select_res.context(DbQuerySnafu {
            table: "users".to_string(),
        })?;

        let items: Vec<UserDto> = items.into_iter().map(|x| x.into()).collect();

        Ok(Paginated::new(
            items,
            pagination.page,
            pagination.per_page,
            pagination.total_records,
        ))
    }

    pub async fn create(&self, data: NewUserDto) -> Result<UserDto> {
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

    pub async fn create_with_password(&self, new_user: NewUserWithPasswordDto) -> Result<UserDto> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        // Expects password to be already hashed
        // let new_user = new_user.clone();

        let trans_res = db
            .interact(move |conn| {
                conn.transaction::<_, Error, _>(|conn| {
                    let today = chrono::Utc::now();
                    let status = "active".to_string();

                    // Create user
                    let user_id = diesel::insert_into(users::table)
                        .values((
                            users::email.eq(new_user.email.clone()),
                            users::name.eq(new_user.name.clone()),
                            users::status.eq(&status),
                            users::created_at.eq(today.clone()),
                            users::updated_at.eq(today.clone()),
                        ))
                        .returning(users::id)
                        .get_result::<i32>(conn)?;

                    // Create password
                    let _ = diesel::insert_into(passwords::table)
                        .values((
                            passwords::id.eq(user_id),
                            passwords::password.eq(new_user.password),
                            passwords::created_at.eq(today.clone()),
                            passwords::updated_at.eq(today.clone()),
                        ))
                        .execute(conn)?;

                    Ok(UserDto {
                        id: user_id,
                        email: new_user.email,
                        name: new_user.name,
                        status: status,
                        created_at: today.to_rfc3339_opts(SecondsFormat::Millis, true),
                        updated_at: today.to_rfc3339_opts(SecondsFormat::Millis, true),
                    })
                })
            })
            .await
            .context(DbInteractSnafu)?;

        let user = trans_res.context(DbQuerySnafu {
            table: "users".to_string(),
        })?;

        Ok(user)
    }

    pub async fn get(&self, id: i32) -> Result<Option<UserDto>> {
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

    pub async fn find_by_email(&self, email: &str) -> Result<Option<UserDto>> {
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

    pub async fn update(&self, id: i32, data: UpdateUserDto) -> Result<bool> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        // Do not allow empty update
        if data.status.is_none() && data.name.is_none() {
            return Ok(false);
        }

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

    pub async fn delete(&self, id: i32) -> Result<bool> {
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
