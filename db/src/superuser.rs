use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;
use diesel::result::Error;
use diesel::{QueryDsl, SelectableHelper};
use snafu::ResultExt;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu};
use crate::schema::org_members;
use crate::schema::orgs;
use crate::schema::passwords;
use crate::schema::superusers::{self, dsl};
use crate::schema::users;
use yaas::dto::{NewPasswordDto, NewUserDto, SuperuserDto};

#[derive(Clone, Queryable, Selectable, Insertable)]
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

pub struct SuperuserRepo {
    db_pool: Pool,
}

impl SuperuserRepo {
    pub fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }

    pub async fn setup(
        &self,
        new_user: NewUserDto,
        new_password: NewPasswordDto,
    ) -> Result<SuperuserDto> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let new_user_copy = new_user.clone();

        // Expects password to be already hashed
        let new_password_copy = new_password.clone();

        let trans_res = db
            .interact(move |conn| {
                conn.transaction::<_, Error, _>(|conn| {
                    let today = chrono::Utc::now();

                    // Create user
                    let user_id = diesel::insert_into(users::table)
                        .values((
                            users::email.eq(new_user_copy.email),
                            users::name.eq(new_user_copy.name),
                            users::status.eq("active"),
                            users::created_at.eq(today),
                            users::updated_at.eq(today),
                        ))
                        .returning(users::id)
                        .get_result::<i32>(conn)?;

                    // Create password
                    let _ = diesel::insert_into(passwords::table)
                        .values((
                            passwords::id.eq(user_id),
                            passwords::password.eq(new_password_copy.password),
                            passwords::created_at.eq(today),
                            passwords::updated_at.eq(today),
                        ))
                        .execute(conn)?;

                    // Create organization
                    let org_id = diesel::insert_into(orgs::table)
                        .values((
                            orgs::name.eq("Superuser"),
                            orgs::status.eq("active"),
                            orgs::owner_id.eq(user_id),
                            orgs::created_at.eq(today),
                            orgs::updated_at.eq(today),
                        ))
                        .returning(orgs::id)
                        .get_result::<i32>(conn)?;

                    // Add as member
                    let _ = diesel::insert_into(org_members::table)
                        .values((
                            org_members::org_id.eq(org_id),
                            org_members::user_id.eq(user_id),
                            org_members::roles.eq("Superuser"),
                            org_members::status.eq("active"),
                            org_members::created_at.eq(today),
                            org_members::updated_at.eq(today),
                        ))
                        .execute(conn)?;

                    // Create superuser entry
                    let _ = diesel::insert_into(superusers::table)
                        .values((
                            superusers::id.eq(user_id),
                            superusers::created_at.eq(today),
                        ))
                        .execute(conn)?;

                    Ok(SuperuserDto {
                        id: user_id,
                        created_at: today.to_rfc3339_opts(SecondsFormat::Millis, true),
                    })
                })
            })
            .await
            .context(DbInteractSnafu)?;

        let superuser = trans_res.context(DbQuerySnafu {
            table: "superusers".to_string(),
        })?;

        Ok(superuser)
    }

    pub async fn list(&self) -> Result<Vec<SuperuserDto>> {
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

    pub async fn create(&self, user_id: i32) -> Result<SuperuserDto> {
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

    pub async fn get(&self, id: i32) -> Result<Option<SuperuserDto>> {
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
