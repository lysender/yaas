use async_trait::async_trait;

use chrono::{DateTime, SecondsFormat, Utc};
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;
use diesel::{QueryDsl, SelectableHelper};
use serde::Deserialize;
use snafu::ResultExt;

use crate::Result;
use crate::error::{DbInteractSnafu, DbPoolSnafu, DbQuerySnafu};
use crate::schema::oauth_codes::{self, dsl};
use yaas::dto::OauthCodeDto;
use yaas::utils::generate_id;

const OAUTH_CODE_ID_PREFIX: &'static str = "oac";

#[derive(Debug, Clone, Queryable, Selectable, Insertable)]
#[diesel(table_name = crate::schema::oauth_codes)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct OauthCode {
    pub id: String,
    pub code: String,
    pub state: String,
    pub redirect_uri: String,
    pub scope: String,
    pub app_id: String,
    pub org_id: String,
    pub user_id: String,
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

#[derive(Debug, Clone, Deserialize)]
pub struct NewOauthCode {
    pub name: String,
    pub code: String,
    pub state: String,
    pub redirect_uri: String,
    pub scope: String,
    pub app_id: String,
    pub org_id: String,
    pub user_id: String,
}

#[async_trait]
pub trait OauthCodeStore: Send + Sync {
    fn generate_id(&self) -> String;

    async fn list(&self) -> Result<Vec<OauthCodeDto>>;

    async fn create(&self, data: &NewOauthCode) -> Result<OauthCodeDto>;

    async fn get(&self, id: &str) -> Result<Option<OauthCodeDto>>;

    async fn delete(&self, id: &str) -> Result<()>;
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
    fn generate_id(&self) -> String {
        generate_id(OAUTH_CODE_ID_PREFIX)
    }

    async fn list(&self) -> Result<Vec<OauthCodeDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let select_res = db
            .interact(move |conn| {
                dsl::oauth_codes
                    .select(OauthCode::as_select())
                    .load::<OauthCode>(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let items = select_res.context(DbQuerySnafu {
            table: "orgs".to_string(),
        })?;

        let items: Vec<OauthCodeDto> = items.into_iter().map(|x| x.into()).collect();

        Ok(items)
    }

    async fn create(&self, data: &NewOauthCode) -> Result<OauthCodeDto> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let data_copy = data.clone();
        let today = chrono::Utc::now();
        let expires_at = today + chrono::Duration::days(7);

        let doc = OauthCode {
            id: generate_id(OAUTH_CODE_ID_PREFIX),
            code: data_copy.code,
            state: data_copy.state,
            redirect_uri: data_copy.redirect_uri,
            scope: data_copy.scope,
            app_id: data_copy.app_id,
            org_id: data_copy.org_id,
            user_id: data_copy.user_id,
            created_at: today.clone(),
            expires_at,
        };

        let doc_copy = doc.clone();
        let inser_res = db
            .interact(move |conn| {
                diesel::insert_into(oauth_codes::table)
                    .values(&doc_copy)
                    .execute(conn)
            })
            .await
            .context(DbInteractSnafu)?;

        let _ = inser_res.context(DbQuerySnafu {
            table: "oauth_codes".to_string(),
        })?;

        Ok(doc.into())
    }

    async fn get(&self, id: &str) -> Result<Option<OauthCodeDto>> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let id = id.to_string();
        let select_res = db
            .interact(move |conn| {
                dsl::oauth_codes
                    .find(&id)
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

    async fn delete(&self, id: &str) -> Result<()> {
        let db = self.db_pool.get().await.context(DbPoolSnafu)?;

        let id = id.to_string();
        let delete_res = db
            .interact(move |conn| {
                diesel::delete(dsl::oauth_codes.filter(dsl::id.eq(&id))).execute(conn)
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
const TEST_OAUTH_CODE_ID: &'static str = "oac_01989be8e9b27912949c4ed5fc548328";

#[cfg(feature = "test")]
pub fn create_test_oauth_code() -> OauthCode {
    use crate::{app::TEST_APP_ID, org::TEST_ORG_ID, user::TEST_USER_ID};

    let today = chrono::Utc::now();

    OauthCode {
        id: TEST_OAUTH_CODE_ID.to_string(),
        code: "test_code".to_string(),
        state: "test_state".to_string(),
        redirect_uri: "https://example.com/callback".to_string(),
        scope: "read write".to_string(),
        app_id: TEST_APP_ID.to_string(),
        org_id: TEST_ORG_ID.to_string(),
        user_id: TEST_USER_ID.to_string(),
        created_at: today.clone(),
        expires_at: today,
    }
}

#[cfg(feature = "test")]
pub struct OauthCodeTestRepo {}

#[cfg(feature = "test")]
#[async_trait]
impl OauthCodeStore for OauthCodeTestRepo {
    fn generate_id(&self) -> String {
        generate_id(OAUTH_CODE_ID_PREFIX)
    }

    async fn list(&self) -> Result<Vec<OauthCodeDto>> {
        let doc1 = create_test_oauth_code();
        let docs = vec![doc1];
        let filtered: Vec<OauthCodeDto> = docs.into_iter().map(|x| x.into()).collect();
        Ok(filtered)
    }

    async fn create(&self, _data: &NewOauthCode) -> Result<OauthCodeDto> {
        Err("Not supported".into())
    }

    async fn get(&self, id: &str) -> Result<Option<OauthCodeDto>> {
        let org1 = create_test_oauth_code();
        let orgs = vec![org1];
        let found = orgs.into_iter().find(|x| x.id.as_str() == id);
        Ok(found.map(|x| x.into()))
    }

    async fn delete(&self, _id: &str) -> Result<()> {
        Ok(())
    }
}
