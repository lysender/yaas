use async_trait::async_trait;

use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;
use diesel::{AsChangeset, QueryDsl, SelectableHelper};
use serde::Deserialize;
use snafu::ResultExt;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu};
use crate::schema::org_members::{self, dsl};
use crate::schema::orgs;
use yaas::role::to_roles;
use yaas::xdto::{OrgMemberDto, OrgMembershipDto};

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

impl From<OrgMember> for OrgMemberDto {
    fn from(org: OrgMember) -> Self {
        let roles = org.roles.split(',').map(|s| s.to_string()).collect();
        let roles = to_roles(&roles).expect("Roles should convert");

        OrgMemberDto {
            id: org.id,
            org_id: org.org_id,
            user_id: org.user_id,
            roles,
            status: org.status,
            created_at: org.created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
            updated_at: org.created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
        }
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

impl From<OrgMembership> for OrgMembershipDto {
    fn from(membership: OrgMembership) -> Self {
        let roles = membership.roles.split(',').map(|s| s.to_string()).collect();
        let roles = to_roles(&roles).expect("Roles should convert");

        OrgMembershipDto {
            org_id: membership.id,
            org_name: membership.name,
            user_id: membership.user_id,
            roles,
        }
    }
}

#[async_trait]
pub trait OrgMemberStore: Send + Sync {
    async fn list(&self, org_id: i32) -> Result<Vec<OrgMemberDto>>;

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
}

#[async_trait]
impl OrgMemberStore for OrgMemberRepo {
    async fn list(&self, org_id: i32) -> Result<Vec<OrgMemberDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let select_res = db
            .interact(move |conn| {
                dsl::org_members
                    .filter(dsl::org_id.eq(org_id))
                    .select(OrgMember::as_select())
                    .load::<OrgMember>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let items = select_res.context(DbQuerySnafu {
            table: "org_members".to_string(),
        })?;

        let items: Vec<OrgMemberDto> = items.into_iter().map(|x| x.into()).collect();

        Ok(items)
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

        let items: Vec<OrgMembershipDto> = items.into_iter().map(|x| x.into()).collect();

        Ok(items)
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

        Ok(doc.into())
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

        Ok(org.map(|x| x.into()))
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
    async fn list(&self, _org_id: i32) -> Result<Vec<OrgMemberDto>> {
        let doc1 = create_test_org_member();
        let docs = vec![doc1];
        let filtered: Vec<OrgMemberDto> = docs.into_iter().map(|x| x.into()).collect();
        Ok(filtered)
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
        Ok(found.map(|x| x.into()))
    }

    async fn update(&self, _id: i32, _data: &UpdateOrgMember) -> Result<bool> {
        Ok(true)
    }

    async fn delete(&self, _id: i32) -> Result<()> {
        Ok(())
    }
}
