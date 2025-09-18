use async_trait::async_trait;
use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::dsl::now;
use diesel::prelude::*;
use diesel::{QueryDsl, SelectableHelper};
use snafu::ResultExt;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu};
use crate::schema::oauth_codes::{self, dsl};
use yaas::dto::{NewOauthCodeDto, OauthCodeDto};

#[derive(Clone, Queryable, Selectable)]
#[diesel(table_name = crate::schema::oauth_codes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OauthCode {
    pub id: i32,
    pub code: String,
    pub state: String,
    pub redirect_uri: String,
    pub scope: String,
    pub app_id: i32,
    pub org_id: i32,
    pub user_id: i32,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::oauth_codes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct InsertableOauthCode {
    pub code: String,
    pub state: String,
    pub redirect_uri: String,
    pub scope: String,
    pub app_id: i32,
    pub org_id: i32,
    pub user_id: i32,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}

impl From<OauthCode> for OauthCodeDto {
    fn from(org: OauthCode) -> Self {
        OauthCodeDto {
            id: org.id,
            code: org.code,
            state: org.state,
            redirect_uri: org.redirect_uri,
            scope: org.scope,
            app_id: org.app_id,
            org_id: org.org_id,
            user_id: org.user_id,
            created_at: org.created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
            expires_at: org.created_at.to_rfc3339_opts(SecondsFormat::Millis, true),
        }
    }
}

#[async_trait]
pub trait OauthCodeStore: Send + Sync {
    async fn list_by_user(&self, user_id: i32) -> Result<Vec<OauthCodeDto>>;

    async fn create(&self, data: NewOauthCodeDto) -> Result<OauthCodeDto>;

    async fn get(&self, id: i32) -> Result<Option<OauthCodeDto>>;

    async fn find_by_code(&self, code: &str) -> Result<Option<OauthCodeDto>>;

    async fn delete(&self, id: i32) -> Result<()>;

    async fn delete_expired(&self) -> Result<()>;
}

pub struct OauthCodeRepo {
    db_pool: Pool,
}

impl OauthCodeRepo {
    pub fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }
}

#[async_trait]
impl OauthCodeStore for OauthCodeRepo {
    async fn list_by_user(&self, user_id: i32) -> Result<Vec<OauthCodeDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        // Ensure we only return non-expired codes
        let select_res = db
            .interact(move |conn| {
                dsl::oauth_codes
                    .filter(dsl::user_id.eq(user_id))
                    .filter(dsl::expires_at.gt(now))
                    .select(OauthCode::as_select())
                    .load::<OauthCode>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let items = select_res.context(DbQuerySnafu {
            table: "oauth_codes".to_string(),
        })?;

        let items: Vec<OauthCodeDto> = items.into_iter().map(|x| x.into()).collect();

        Ok(items)
    }

    async fn create(&self, data: NewOauthCodeDto) -> Result<OauthCodeDto> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let today = chrono::Utc::now();
        let expires_at = today + chrono::Duration::days(7);

        let new_doc = InsertableOauthCode {
            code: data.code,
            state: data.state,
            redirect_uri: data.redirect_uri,
            scope: data.scope,
            app_id: data.app_id,
            org_id: data.org_id,
            user_id: data.user_id,
            created_at: today.clone(),
            expires_at,
        };

        let doc_copy = new_doc.clone();
        let inser_res = db
            .interact(move |conn| {
                diesel::insert_into(oauth_codes::table)
                    .values(&doc_copy)
                    .returning(oauth_codes::id)
                    .get_result(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let id: i32 = inser_res.context(DbQuerySnafu {
            table: "oauth_codes".to_string(),
        })?;

        let doc = OauthCode {
            id,
            code: new_doc.code,
            state: new_doc.state,
            redirect_uri: new_doc.redirect_uri,
            scope: new_doc.scope,
            app_id: new_doc.app_id,
            org_id: new_doc.org_id,
            user_id: new_doc.user_id,
            created_at: new_doc.created_at,
            expires_at: new_doc.expires_at,
        };

        Ok(doc.into())
    }

    async fn get(&self, id: i32) -> Result<Option<OauthCodeDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        // Ensure we only return non-expired codes
        let select_res = db
            .interact(move |conn| {
                dsl::oauth_codes
                    .filter(dsl::id.eq(id))
                    .filter(dsl::expires_at.gt(now))
                    .select(OauthCode::as_select())
                    .first::<OauthCode>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let org = select_res.context(DbQuerySnafu {
            table: "oauth_codes".to_string(),
        })?;

        Ok(org.map(|x| x.into()))
    }

    async fn find_by_code(&self, code: &str) -> Result<Option<OauthCodeDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let code_str = code.to_string();

        // Ensure we only return non-expired codes
        let select_res = db
            .interact(move |conn| {
                dsl::oauth_codes
                    .filter(dsl::code.eq(code_str))
                    .filter(dsl::expires_at.gt(now))
                    .select(OauthCode::as_select())
                    .first::<OauthCode>(conn)
                    .optional()
            })
            .await
            .context(DbInteractSnafu)?;

        let org = select_res.context(DbQuerySnafu {
            table: "oauth_codes".to_string(),
        })?;

        Ok(org.map(|x| x.into()))
    }

    async fn delete(&self, id: i32) -> Result<()> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let delete_res = db
            .interact(move |conn| {
                diesel::delete(dsl::oauth_codes.filter(dsl::id.eq(id))).execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let _ = delete_res.context(DbQuerySnafu {
            table: "oauth_codes".to_string(),
        })?;

        Ok(())
    }

    async fn delete_expired(&self) -> Result<()> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let delete_res = db
            .interact(move |conn| {
                diesel::delete(dsl::oauth_codes.filter(dsl::expires_at.le(now))).execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let _ = delete_res.context(DbQuerySnafu {
            table: "oauth_codes".to_string(),
        })?;

        Ok(())
    }
}

#[cfg(feature = "test")]
const TEST_OAUTH_CODE_ID: i32 = 6000;

#[cfg(feature = "test")]
pub fn create_test_oauth_code() -> OauthCode {
    use crate::{app::TEST_APP_ID, org::TEST_ORG_ID, user::TEST_USER_ID};

    let today = chrono::Utc::now();

    OauthCode {
        id: TEST_OAUTH_CODE_ID,
        code: "test_code".to_string(),
        state: "test_state".to_string(),
        redirect_uri: "https://example.com/callback".to_string(),
        scope: "read write".to_string(),
        app_id: TEST_APP_ID,
        org_id: TEST_ORG_ID,
        user_id: TEST_USER_ID,
        created_at: today.clone(),
        expires_at: today,
    }
}

#[cfg(feature = "test")]
pub struct OauthCodeTestRepo {}

#[cfg(feature = "test")]
#[async_trait]
impl OauthCodeStore for OauthCodeTestRepo {
    async fn list_by_user(&self, user_id: i32) -> Result<Vec<OauthCodeDto>> {
        let doc1 = create_test_oauth_code();
        let docs = vec![doc1];
        let filtered: Vec<OauthCodeDto> = docs
            .into_iter()
            .filter(|x| x.user_id == user_id)
            .map(|x| x.into())
            .collect();
        Ok(filtered)
    }

    async fn create(&self, _data: NewOauthCodeDto) -> Result<OauthCodeDto> {
        Err("Not supported".into())
    }

    async fn get(&self, id: i32) -> Result<Option<OauthCodeDto>> {
        let org1 = create_test_oauth_code();
        let orgs = vec![org1];
        let found = orgs.into_iter().find(|x| x.id == id);
        Ok(found.map(|x| x.into()))
    }

    async fn find_by_code(&self, code: &str) -> Result<Option<OauthCodeDto>> {
        let org1 = create_test_oauth_code();
        let orgs = vec![org1];
        let found = orgs.into_iter().find(|x| x.code == code);
        Ok(found.map(|x| x.into()))
    }

    async fn delete(&self, _id: i32) -> Result<()> {
        Ok(())
    }

    async fn delete_expired(&self) -> Result<()> {
        Ok(())
    }
}
