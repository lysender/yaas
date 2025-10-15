use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::dsl::count_star;
use diesel::prelude::*;
use diesel::{AsChangeset, QueryDsl};
use serde::Deserialize;
use snafu::ResultExt;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu};
use crate::schema::org_members::{self, dsl};
use crate::schema::orgs;
use crate::schema::users;
use yaas::dto::{
    ListOrgMembersParamsDto, NewOrgMemberDto, OrgMemberDto, OrgMemberSuggestionDto,
    OrgMembershipDto, UpdateOrgMemberDto,
};
use yaas::pagination::{ListingParamsDto, Paginated, PaginationParams};
use yaas::role::{Role, to_roles};

#[derive(Clone, Queryable, Selectable)]
#[diesel(table_name = crate::schema::org_members)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OrgMember {
    pub id: i32,
    pub org_id: i32,
    pub user_id: i32,
    pub roles: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Queryable)]
pub struct OrgMemberWithName {
    pub id: i32,
    pub org_id: i32,
    pub user_id: i32,
    pub member_email: Option<String>,
    pub member_name: Option<String>,
    pub roles: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::org_members)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct InsertableOrgMember {
    pub org_id: i32,
    pub user_id: i32,
    pub roles: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TryFrom<OrgMember> for OrgMemberDto {
    type Error = String;

    fn try_from(member: OrgMember) -> std::result::Result<Self, Self::Error> {
        let mut roles: Vec<Role> = Vec::new();
        if member.roles.len() > 0 {
            let converted_roles = member.roles.split(',').map(|s| s.to_string()).collect();
            let Ok(converted_roles) = to_roles(&converted_roles) else {
                return Err("Roles should convert back to enum".to_string());
            };
            roles = converted_roles;
        }

        Ok(OrgMemberDto {
            id: member.id,
            org_id: member.org_id,
            user_id: member.user_id,
            member_email: None,
            member_name: None,
            roles,
            status: member.status,
            created_at: member
                .created_at
                .to_rfc3339_opts(SecondsFormat::Millis, true),
            updated_at: member
                .created_at
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        })
    }
}

impl TryFrom<OrgMemberWithName> for OrgMemberDto {
    type Error = String;

    fn try_from(member: OrgMemberWithName) -> std::result::Result<Self, Self::Error> {
        let mut roles: Vec<Role> = Vec::new();
        if member.roles.len() > 0 {
            let converted_roles = member.roles.split(',').map(|s| s.to_string()).collect();
            let Ok(converted_roles) = to_roles(&converted_roles) else {
                return Err("Roles should convert back to enum".to_string());
            };
            roles = converted_roles;
        }

        Ok(OrgMemberDto {
            id: member.id,
            org_id: member.org_id,
            user_id: member.user_id,
            member_email: member.member_email,
            member_name: member.member_name,
            roles,
            status: member.status,
            created_at: member
                .created_at
                .to_rfc3339_opts(SecondsFormat::Millis, true),
            updated_at: member
                .created_at
                .to_rfc3339_opts(SecondsFormat::Millis, true),
        })
    }
}

#[derive(Clone, Deserialize, AsChangeset)]
#[diesel(table_name = crate::schema::org_members)]
pub struct UpdateOrgMember {
    pub roles: Option<String>,
    pub status: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Queryable)]
pub struct OrgMembership {
    pub id: i32,
    pub name: String,
    pub user_id: i32,
    pub roles: String,
}

impl TryFrom<OrgMembership> for OrgMembershipDto {
    type Error = String;

    fn try_from(membership: OrgMembership) -> std::result::Result<Self, Self::Error> {
        let roles = membership.roles.split(',').map(|s| s.to_string()).collect();
        let Ok(roles) = to_roles(&roles) else {
            return Err("Roles should convert back to enum".to_string());
        };

        Ok(OrgMembershipDto {
            org_id: membership.id,
            org_name: membership.name,
            user_id: membership.user_id,
            roles,
        })
    }
}

#[derive(Queryable)]
pub struct OrgMemberSuggestion {
    pub id: i32,
    pub name: String,
    pub email: String,
}

impl From<OrgMemberSuggestion> for OrgMemberSuggestionDto {
    fn from(suggestion: OrgMemberSuggestion) -> Self {
        OrgMemberSuggestionDto {
            id: suggestion.id,
            name: suggestion.name,
            email: suggestion.email,
        }
    }
}

pub struct OrgMemberRepo {
    db_pool: Pool,
}

impl OrgMemberRepo {
    pub fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }

    async fn list_count(&self, org_id: i32, params: ListOrgMembersParamsDto) -> Result<i64> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let count_res = db
            .interact(move |conn| {
                let mut query = dsl::org_members
                    .left_outer_join(users::table.on(users::id.eq(org_members::user_id)))
                    .into_boxed();

                if let Some(keyword) = params.keyword {
                    if keyword.len() > 0 {
                        let pattern = format!("%{}%", keyword);
                        query = query.filter(
                            users::name
                                .ilike(pattern.clone())
                                .or(users::email.ilike(pattern)),
                        );
                    }
                }

                query
                    .filter(dsl::org_id.eq(org_id))
                    .filter(users::deleted_at.is_null())
                    .select(count_star())
                    .get_result::<i64>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let count = count_res.context(DbQuerySnafu {
            table: "org_members".to_string(),
        })?;

        Ok(count)
    }

    pub async fn list(
        &self,
        org_id: i32,
        params: ListOrgMembersParamsDto,
    ) -> Result<Paginated<OrgMemberDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let total_records = self.list_count(org_id, params.clone()).await?;

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
                let mut query = dsl::org_members
                    .left_outer_join(users::table.on(users::id.eq(org_members::user_id)))
                    .into_boxed();

                if let Some(keyword) = params.keyword {
                    if keyword.len() > 0 {
                        let pattern = format!("%{}%", keyword);
                        query = query.filter(
                            users::name
                                .ilike(pattern.clone())
                                .or(users::email.ilike(pattern)),
                        );
                    }
                }

                query
                    .filter(dsl::org_id.eq(org_id))
                    .filter(users::deleted_at.is_null())
                    .order_by(users::email.asc())
                    .limit(pagination.per_page as i64)
                    .offset(pagination.offset)
                    .select((
                        org_members::id,
                        org_members::org_id,
                        org_members::user_id,
                        users::email.nullable(),
                        users::name.nullable(),
                        org_members::roles,
                        org_members::status,
                        org_members::created_at,
                        org_members::updated_at,
                    ))
                    .load::<OrgMemberWithName>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let items = select_res.context(DbQuerySnafu {
            table: "org_members".to_string(),
        })?;

        let items: std::result::Result<Vec<OrgMemberDto>, String> =
            items.into_iter().map(|x| x.try_into()).collect();

        match items {
            Ok(list) => Ok(Paginated::new(
                list,
                pagination.page,
                pagination.per_page,
                pagination.total_records,
            )),
            Err(e) => Err(e.into()),
        }
    }

    async fn list_memberships_count(&self, user_id: i32) -> Result<i64> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let count_res = db
            .interact(move |conn| {
                orgs::table
                    .inner_join(org_members::table.on(orgs::id.eq(org_members::org_id)))
                    .filter(orgs::status.eq("active"))
                    .filter(orgs::deleted_at.is_null())
                    .filter(org_members::status.eq("active"))
                    .filter(org_members::user_id.eq(user_id))
                    .select(count_star())
                    .get_result::<i64>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let count = count_res.context(DbQuerySnafu {
            table: "org_members".to_string(),
        })?;

        Ok(count)
    }

    pub async fn list_memberships(
        &self,
        user_id: i32,
        params: ListingParamsDto,
    ) -> Result<Paginated<OrgMembershipDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let total_records = self.list_memberships_count(user_id).await?;

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
                orgs::table
                    .inner_join(org_members::table.on(orgs::id.eq(org_members::org_id)))
                    .filter(orgs::status.eq("active"))
                    .filter(orgs::deleted_at.is_null())
                    .filter(org_members::status.eq("active"))
                    .filter(org_members::user_id.eq(user_id))
                    .order_by(orgs::name.asc())
                    .limit(pagination.per_page as i64)
                    .offset(pagination.offset)
                    .select((
                        orgs::id,
                        orgs::name,
                        org_members::user_id,
                        org_members::roles,
                    ))
                    .load::<OrgMembership>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let items = select_res.context(DbQuerySnafu {
            table: "org_members".to_string(),
        })?;

        let items: std::result::Result<Vec<OrgMembershipDto>, String> =
            items.into_iter().map(|x| x.try_into()).collect();

        match items {
            Ok(list) => Ok(Paginated::new(
                list,
                pagination.page,
                pagination.per_page,
                pagination.total_records,
            )),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn create(&self, org_id: i32, data: NewOrgMemberDto) -> Result<OrgMemberDto> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let today = chrono::Utc::now();

        let new_doc = InsertableOrgMember {
            org_id,
            user_id: data.user_id,
            roles: data.roles.join(","),
            status: data.status,
            created_at: today.clone(),
            updated_at: today,
        };

        let doc_copy = new_doc.clone();
        let inser_res = db
            .interact(move |conn| {
                diesel::insert_into(org_members::table)
                    .values(doc_copy)
                    .returning(org_members::id)
                    .get_result(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let id: i32 = inser_res.context(DbQuerySnafu {
            table: "org_members".to_string(),
        })?;

        let doc = OrgMember {
            id,
            org_id: new_doc.org_id,
            user_id: new_doc.user_id,
            roles: new_doc.roles,
            status: new_doc.status,
            created_at: new_doc.created_at,
            updated_at: new_doc.updated_at,
        };

        match doc.try_into() {
            Ok(m) => Ok(m),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn get(&self, id: i32) -> Result<Option<OrgMemberDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let select_res = db
            .interact(move |conn| {
                dsl::org_members
                    .left_outer_join(users::table.on(users::id.eq(org_members::user_id)))
                    .filter(dsl::id.eq(id))
                    .select((
                        org_members::id,
                        org_members::org_id,
                        org_members::user_id,
                        users::email.nullable(),
                        users::name.nullable(),
                        org_members::roles,
                        org_members::status,
                        org_members::created_at,
                        org_members::updated_at,
                    ))
                    .first::<OrgMemberWithName>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let org = select_res.context(DbQuerySnafu {
            table: "org_members".to_string(),
        })?;

        match org {
            Some(m) => match m.try_into() {
                Ok(m) => Ok(Some(m)),
                Err(e) => Err(e.into()),
            },
            None => Ok(None),
        }
    }

    pub async fn find_member(&self, org_id: i32, user_id: i32) -> Result<Option<OrgMemberDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let select_res = db
            .interact(move |conn| {
                dsl::org_members
                    .left_outer_join(users::table.on(users::id.eq(org_members::user_id)))
                    .filter(dsl::org_id.eq(org_id))
                    .filter(dsl::user_id.eq(user_id))
                    .select((
                        org_members::id,
                        org_members::org_id,
                        org_members::user_id,
                        users::email.nullable(),
                        users::name.nullable(),
                        org_members::roles,
                        org_members::status,
                        org_members::created_at,
                        org_members::updated_at,
                    ))
                    .first::<OrgMemberWithName>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let org = select_res.context(DbQuerySnafu {
            table: "org_members".to_string(),
        })?;

        match org {
            Some(m) => match m.try_into() {
                Ok(m) => Ok(Some(m)),
                Err(e) => Err(e.into()),
            },
            None => Ok(None),
        }
    }

    async fn list_member_suggestions_count(
        &self,
        org_id: i32,
        params: ListOrgMembersParamsDto,
    ) -> Result<i64> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let count_res = db
            .interact(move |conn| {
                let mut query = users::dsl::users
                    .left_outer_join(
                        org_members::table.on(org_members::user_id
                            .eq(users::id)
                            .and(users::deleted_at.is_null())
                            .and(dsl::org_id.eq(org_id))),
                    )
                    .into_boxed();

                if let Some(keyword) = params.keyword {
                    if keyword.len() > 0 {
                        let pattern = format!("%{}%", keyword);
                        query = query.filter(
                            users::name
                                .ilike(pattern.clone())
                                .or(users::email.ilike(pattern)),
                        );
                    }
                }

                query
                    .filter(org_members::user_id.is_null())
                    .select(count_star())
                    .get_result::<i64>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let count = count_res.context(DbQuerySnafu {
            table: "org_members".to_string(),
        })?;

        Ok(count)
    }

    pub async fn list_member_suggestions(
        &self,
        org_id: i32,
        params: ListOrgMembersParamsDto,
    ) -> Result<Paginated<OrgMemberSuggestionDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let total_records = self
            .list_member_suggestions_count(org_id, params.clone())
            .await?;

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
                let mut query = users::dsl::users
                    .left_outer_join(
                        org_members::table.on(org_members::user_id
                            .eq(users::id)
                            .and(users::deleted_at.is_null())
                            .and(dsl::org_id.eq(org_id))),
                    )
                    .into_boxed();

                if let Some(keyword) = params.keyword {
                    if keyword.len() > 0 {
                        let pattern = format!("%{}%", keyword);
                        query = query.filter(
                            users::name
                                .ilike(pattern.clone())
                                .or(users::email.ilike(pattern)),
                        );
                    }
                }

                query
                    .filter(org_members::user_id.is_null())
                    .order_by(users::email.asc())
                    .select((users::id, users::name, users::email))
                    .load::<OrgMemberSuggestion>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let items = select_res.context(DbQuerySnafu {
            table: "org_members".to_string(),
        })?;

        let items: Vec<OrgMemberSuggestionDto> = items.into_iter().map(|x| x.into()).collect();
        Ok(Paginated::new(
            items,
            pagination.page,
            pagination.per_page,
            pagination.total_records,
        ))
    }

    pub async fn update(&self, id: i32, data: UpdateOrgMemberDto) -> Result<bool> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        // Do not allow empty update
        if data.status.is_none() && data.roles.is_none() {
            return Ok(false);
        }

        let updated_member = UpdateOrgMember {
            roles: data.roles.map(|r| r.join(",")),
            status: data.status,
            updated_at: Some(chrono::Utc::now()),
        };

        let update_res = db
            .interact(move |conn| {
                diesel::update(dsl::org_members)
                    .filter(dsl::id.eq(id))
                    .set(updated_member)
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let affected = update_res.context(DbQuerySnafu {
            table: "org_members".to_string(),
        })?;

        Ok(affected > 0)
    }

    pub async fn delete(&self, id: i32) -> Result<()> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let delete_res = db
            .interact(move |conn| {
                diesel::delete(dsl::org_members.filter(dsl::id.eq(id))).execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let _ = delete_res.context(DbQuerySnafu {
            table: "org_members".to_string(),
        })?;

        Ok(())
    }
}
