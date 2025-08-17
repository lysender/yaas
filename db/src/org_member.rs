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
use yaas::dto::OrgMemberDto;
use yaas::utils::generate_id;

const ORG_MEMBER_ID_PREFIX: &'static str = "orm";

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::org_members)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OrgMember {
    pub id: String,
    pub org_id: String,
    pub user_id: String,
    pub roles: Vec<Option<String>>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<OrgMember> for OrgMemberDto {
    fn from(org: OrgMember) -> Self {
        OrgMemberDto {
            id: org.id,
            org_id: org.org_id,
            user_id: org.user_id,
            roles: org.roles.into_iter().filter_map(|x| x).collect(),
            status: org.status,
            created_at: org.created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
            updated_at: org.created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewOrgMember {
    pub user_id: String,
    pub roles: Vec<String>,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, AsChangeset)]
#[diesel(table_name = crate::schema::org_members)]
pub struct UpdateOrgMember {
    pub roles: Option<Vec<String>>,
    pub status: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[async_trait]
pub trait OrgMemberStore: Send + Sync {
    fn generate_id(&self) -> String;

    async fn list(&self, org_id: &str) -> Result<Vec<OrgMemberDto>>;

    async fn create(&self, org_id: &str, data: &NewOrgMember) -> Result<OrgMemberDto>;

    async fn get(&self, id: &str) -> Result<Option<OrgMemberDto>>;

    async fn update(&self, id: &str, data: &UpdateOrgMember) -> Result<bool>;

    async fn delete(&self, id: &str) -> Result<()>;
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
    fn generate_id(&self) -> String {
        generate_id(ORG_MEMBER_ID_PREFIX)
    }

    async fn list(&self, org_id: &str) -> Result<Vec<OrgMemberDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;
        let org_id = org_id.to_string();

        let select_res = db
            .interact(move |conn| {
                dsl::org_members
                    .filter(dsl::org_id.eq(&org_id))
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

    async fn create(&self, org_id: &str, data: &NewOrgMember) -> Result<OrgMemberDto> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let data_copy = data.clone();
        let today = chrono::Utc::now();

        let doc = OrgMember {
            id: generate_id(ORG_MEMBER_ID_PREFIX),
            org_id: org_id.to_string(),
            user_id: data_copy.user_id,
            roles: data_copy.roles.into_iter().map(Some).collect(),
            status: data_copy.status,
            created_at: today.clone(),
            updated_at: today,
        };

        let doc_copy = doc.clone();
        let inser_res = db
            .interact(move |conn| {
                diesel::insert_into(org_members::table)
                    .values(&doc_copy)
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let _ = inser_res.context(DbQuerySnafu {
            table: "org_members".to_string(),
        })?;

        Ok(doc.into())
    }

    async fn get(&self, id: &str) -> Result<Option<OrgMemberDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let id = id.to_string();
        let select_res = db
            .interact(move |conn| {
                dsl::org_members
                    .find(&id)
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

    async fn update(&self, id: &str, data: &UpdateOrgMember) -> Result<bool> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let id = id.to_string();
        let mut data_clone = data.clone();
        if data_clone.updated_at.is_none() {
            data_clone.updated_at = Some(chrono::Utc::now());
        }
        let update_res = db
            .interact(move |conn| {
                diesel::update(dsl::org_members)
                    .filter(dsl::id.eq(&id))
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

    async fn delete(&self, id: &str) -> Result<()> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let id = id.to_string();
        let delete_res = db
            .interact(move |conn| {
                diesel::delete(dsl::org_members.filter(dsl::id.eq(&id))).execute(conn)
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
pub const TEST_ORG_MEMBER_ID: &'static str = "orm_019896b7c4e97c3498b9bd9264266024";

#[cfg(feature = "test")]
pub fn create_test_org_member() -> OrgMember {
    use crate::{org::TEST_ORG_ID, user::TEST_USER_ID};

    let today = chrono::Utc::now();

    OrgMember {
        id: TEST_ORG_MEMBER_ID.to_string(),
        org_id: TEST_ORG_ID.to_string(),
        user_id: TEST_USER_ID.to_string(),
        roles: vec![Some("Admin".to_string())],
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
    fn generate_id(&self) -> String {
        generate_id(ORG_MEMBER_ID_PREFIX)
    }

    async fn list(&self, _org_id: &str) -> Result<Vec<OrgMemberDto>> {
        let doc1 = create_test_org_member();
        let docs = vec![doc1];
        let filtered: Vec<OrgMemberDto> = docs.into_iter().map(|x| x.into()).collect();
        Ok(filtered)
    }

    async fn create(&self, _org_id: &str, _data: &NewOrgMember) -> Result<OrgMemberDto> {
        Err("Not supported".into())
    }

    async fn get(&self, id: &str) -> Result<Option<OrgMemberDto>> {
        let doc1 = create_test_org_member();
        let docs = vec![doc1];
        let found = docs.into_iter().find(|x| x.id.as_str() == id);
        Ok(found.map(|x| x.into()))
    }

    async fn update(&self, _id: &str, _data: &UpdateOrgMember) -> Result<bool> {
        Ok(true)
    }

    async fn delete(&self, _id: &str) -> Result<()> {
        Ok(())
    }
}
