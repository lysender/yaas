use async_trait::async_trait;

use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::dsl::count_star;
use diesel::prelude::*;
use diesel::{AsChangeset, QueryDsl, SelectableHelper};
use serde::Deserialize;
use snafu::ResultExt;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu};
use crate::schema::org_members::{self, dsl};
use crate::schema::orgs;
use crate::schema::users;
use yaas::dto::{ListOrgMembersParamsDto, OrgMemberDto, OrgMembershipDto};
use yaas::pagination::{Paginated, PaginationParams};
use yaas::role::to_roles;

#[derive(Debug, Clone, Queryable, Selectable)]
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
    pub name: Option<String>,
    pub roles: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
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
        let roles = member.roles.split(',').map(|s| s.to_string()).collect();
        let Ok(roles) = to_roles(&roles) else {
            return Err("Roles should convert back to enum".to_string());
        };

        Ok(OrgMemberDto {
            id: member.id,
            org_id: member.org_id,
            user_id: member.user_id,
            name: None,
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
        let roles = member.roles.split(',').map(|s| s.to_string()).collect();
        let Ok(roles) = to_roles(&roles) else {
            return Err("Roles should convert back to enum".to_string());
        };

        Ok(OrgMemberDto {
            id: member.id,
            org_id: member.org_id,
            user_id: member.user_id,
            name: None,
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

#[derive(Debug, Clone, Deserialize)]
pub struct NewOrgMember {
    pub user_id: i32,
    pub roles: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, AsChangeset)]
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

#[async_trait]
pub trait OrgMemberStore: Send + Sync {
    async fn list(
        &self,
        org_id: i32,
        params: ListOrgMembersParamsDto,
    ) -> Result<Paginated<OrgMemberDto>>;

    async fn list_memberships(&self, user_id: i32) -> Result<Vec<OrgMembershipDto>>;

    async fn create(&self, org_id: i32, data: &NewOrgMember) -> Result<OrgMemberDto>;

    async fn get(&self, id: i32) -> Result<Option<OrgMemberDto>>;

    async fn update(&self, id: i32, data: &UpdateOrgMember) -> Result<bool>;

    async fn delete(&self, id: i32) -> Result<()>;
}

pub struct OrgMemberRepo {
    db_pool: Pool,
}

impl OrgMemberRepo {
    pub fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }

    async fn listing_count(&self, org_id: i32, params: ListOrgMembersParamsDto) -> Result<i64> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let count_res = db
            .interact(move |conn| {
                let mut query = dsl::org_members
                    .left_outer_join(users::table.on(users::id.eq(org_members::user_id)))
                    .into_boxed();

                if let Some(keyword) = params.keyword {
                    if keyword.len() > 0 {
                        let pattern = format!("%{}%", keyword);
                        query = query.filter(users::name.like(pattern));
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
            table: "orgs".to_string(),
        })?;

        Ok(count)
    }
}

#[async_trait]
impl OrgMemberStore for OrgMemberRepo {
    async fn list(
        &self,
        org_id: i32,
        params: ListOrgMembersParamsDto,
    ) -> Result<Paginated<OrgMemberDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let total_records = self.listing_count(org_id, params.clone()).await?;

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
                        query = query.filter(users::name.like(pattern));
                    }
                }

                query
                    .filter(dsl::org_id.eq(org_id))
                    .filter(users::deleted_at.is_null())
                    .order_by(users::name.asc())
                    .select((
                        org_members::id,
                        org_members::org_id,
                        org_members::user_id,
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

    async fn list_memberships(&self, user_id: i32) -> Result<Vec<OrgMembershipDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let select_res = db
            .interact(move |conn| {
                orgs::table
                    .inner_join(org_members::table.on(orgs::id.eq(org_members::org_id)))
                    .filter(orgs::status.eq("active"))
                    .filter(orgs::deleted_at.is_null())
                    .filter(org_members::status.eq("active"))
                    .filter(org_members::user_id.eq(user_id))
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

        let list: std::result::Result<Vec<OrgMembershipDto>, String> =
            items.into_iter().map(|x| x.try_into()).collect();

        match list {
            Ok(list) => Ok(list),
            Err(e) => Err(e.into()),
        }
    }

    async fn create(&self, org_id: i32, data: &NewOrgMember) -> Result<OrgMemberDto> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let data_copy = data.clone();
        let today = chrono::Utc::now();

        let new_doc = InsertableOrgMember {
            org_id: org_id,
            user_id: data_copy.user_id,
            roles: data_copy.roles.join(","),
            status: data_copy.status,
            created_at: today.clone(),
            updated_at: today,
        };

        let doc_copy = new_doc.clone();
        let inser_res = db
            .interact(move |conn| {
                diesel::insert_into(org_members::table)
                    .values(&doc_copy)
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

    async fn get(&self, id: i32) -> Result<Option<OrgMemberDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let select_res = db
            .interact(move |conn| {
                dsl::org_members
                    .find(id)
                    .select(OrgMember::as_select())
                    .first::<OrgMember>(conn)
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

    async fn update(&self, id: i32, data: &UpdateOrgMember) -> Result<bool> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let mut data_clone = data.clone();
        if data_clone.updated_at.is_none() {
            data_clone.updated_at = Some(chrono::Utc::now());
        }
        let update_res = db
            .interact(move |conn| {
                diesel::update(dsl::org_members)
                    .filter(dsl::id.eq(id))
                    .set(data_clone)
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let affected = update_res.context(DbQuerySnafu {
            table: "org_members".to_string(),
        })?;

        Ok(affected > 0)
    }

    async fn delete(&self, id: i32) -> Result<()> {
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

#[cfg(feature = "test")]
pub const TEST_ORG_MEMBER_ID: i32 = 5000;

#[cfg(feature = "test")]
pub fn create_test_org_member() -> OrgMember {
    use crate::{org::TEST_ORG_ID, user::TEST_USER_ID};

    let today = chrono::Utc::now();

    OrgMember {
        id: TEST_ORG_MEMBER_ID,
        org_id: TEST_ORG_ID,
        user_id: TEST_USER_ID,
        roles: "Admin".to_string(),
        status: "active".to_string(),
        created_at: today.clone(),
        updated_at: today,
    }
}

#[cfg(feature = "test")]
pub struct OrgMemberTestRepo {}

#[cfg(feature = "test")]
#[async_trait]
impl OrgMemberStore for OrgMemberTestRepo {
    async fn list(
        &self,
        _org_id: i32,
        _params: ListOrgMembersParamsDto,
    ) -> Result<Paginated<OrgMemberDto>> {
        let doc1 = create_test_org_member();
        let docs = vec![doc1];
        let total_records = docs.len() as i64;
        let filtered: std::result::Result<Vec<OrgMemberDto>, String> =
            docs.into_iter().map(|x| x.try_into()).collect();

        match filtered {
            Ok(list) => Ok(Paginated::new(list, 1, 10, total_records)),
            Err(e) => Err(e.into()),
        }
    }

    async fn list_memberships(&self, _user_id: i32) -> Result<Vec<OrgMembershipDto>> {
        use crate::org::create_test_org;

        let org = create_test_org();
        let doc1 = create_test_org_member();
        let docs = vec![doc1];
        let filtered: Vec<OrgMembershipDto> = docs
            .into_iter()
            .map(|x| {
                let roles = x.roles.split(',').map(|s| s.to_string()).collect();
                let roles = to_roles(&roles).expect("Roles should convert");
                return OrgMembershipDto {
                    org_id: org.id,
                    org_name: org.name.clone(),
                    user_id: x.user_id,
                    roles,
                };
            })
            .collect();
        Ok(filtered)
    }

    async fn create(&self, _org_id: i32, _data: &NewOrgMember) -> Result<OrgMemberDto> {
        Err("Not supported".into())
    }

    async fn get(&self, id: i32) -> Result<Option<OrgMemberDto>> {
        let doc1 = create_test_org_member();
        let docs = vec![doc1];
        let found = docs.into_iter().find(|x| x.id == id);
        match found {
            Some(m) => match m.try_into() {
                Ok(m) => Ok(Some(m)),
                Err(e) => Err(e.into()),
            },
            None => Ok(None),
        }
    }

    async fn update(&self, _id: i32, _data: &UpdateOrgMember) -> Result<bool> {
        Ok(true)
    }

    async fn delete(&self, _id: i32) -> Result<()> {
        Ok(())
    }
}
