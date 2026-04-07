use snafu::ResultExt;
use turso::{Connection, Row};

use crate::Result;
use crate::db::turso_decode::{FromTursoRow, collect_row, collect_rows, row_integer, row_text};
use crate::db::turso_params::{integer_param, new_query_params, text_param};
use crate::dto::{NewOauthCodeDto, OauthCodeDto};
use crate::error::{DbPrepareSnafu, DbStatementSnafu};
use crate::utils::{IdPrefix, generate_id};

impl FromTursoRow for OauthCodeDto {
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row_text(row, 0)?,
            code: row_text(row, 1)?,
            state: row_text(row, 2)?,
            redirect_uri: row_text(row, 3)?,
            scope: row_text(row, 4)?,
            app_id: row_text(row, 5)?,
            org_id: row_text(row, 6)?,
            user_id: row_text(row, 7)?,
            created_at: row_integer(row, 8)?,
            expires_at: row_integer(row, 9)?,
        })
    }
}

pub struct OauthCodeRepo {
    db_pool: Connection,
}

impl OauthCodeRepo {
    pub fn new(db_pool: Connection) -> Self {
        Self { db_pool }
    }

    pub async fn list_by_user(&self, user_id: String) -> Result<Vec<OauthCodeDto>> {
        let query = r#"
            SELECT
                id,
                code,
                state,
                redirect_uri,
                scope,
                app_id,
                org_id,
                user_id,
                created_at,
                expires_at
            FROM oauth_codes
            WHERE
                user_id = :user_id
                AND expires_at > :now
            ORDER BY created_at DESC
        "#;

        let now = chrono::Utc::now().timestamp_millis();

        let mut q_params = new_query_params();
        q_params.push(text_param(":user_id", user_id));
        q_params.push(integer_param(":now", now));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let mut rows = stmt.query(q_params).await.context(DbStatementSnafu)?;
        let items: Vec<OauthCodeDto> = collect_rows(&mut rows).await?;
        Ok(items)
    }

    pub async fn create(&self, data: NewOauthCodeDto) -> Result<OauthCodeDto> {
        let query = r#"
            INSERT INTO oauth_codes
            (
                id,
                code,
                state,
                redirect_uri,
                scope,
                app_id,
                org_id,
                user_id,
                created_at,
                expires_at
            )
            VALUES
            (
                :id,
                :code,
                :state,
                :redirect_uri,
                :scope,
                :app_id,
                :org_id,
                :user_id,
                :created_at,
                :expires_at
            )
        "#;

        let id = generate_id(IdPrefix::OauthCode);
        let created_at = chrono::Utc::now().timestamp_millis();
        let expires_at = created_at + chrono::Duration::days(7).num_milliseconds();

        let mut q_params = new_query_params();
        q_params.push(text_param(":id", id.clone()));
        q_params.push(text_param(":code", data.code.clone()));
        q_params.push(text_param(":state", data.state.clone()));
        q_params.push(text_param(":redirect_uri", data.redirect_uri.clone()));
        q_params.push(text_param(":scope", data.scope.clone()));
        q_params.push(text_param(":app_id", data.app_id.clone()));
        q_params.push(text_param(":org_id", data.org_id.clone()));
        q_params.push(text_param(":user_id", data.user_id.clone()));
        q_params.push(integer_param(":created_at", created_at));
        q_params.push(integer_param(":expires_at", expires_at));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let affected = stmt.execute(q_params).await.context(DbStatementSnafu)?;
        assert!(affected > 0, "Must insert a new row");

        Ok(OauthCodeDto {
            id,
            code: data.code,
            state: data.state,
            redirect_uri: data.redirect_uri,
            scope: data.scope,
            app_id: data.app_id,
            org_id: data.org_id,
            user_id: data.user_id,
            created_at,
            expires_at,
        })
    }

    pub async fn get(&self, id: String) -> Result<Option<OauthCodeDto>> {
        let query = r#"
            SELECT
                id,
                code,
                state,
                redirect_uri,
                scope,
                app_id,
                org_id,
                user_id,
                created_at,
                expires_at
            FROM oauth_codes
            WHERE
                id = :id
                AND expires_at > :now
            LIMIT 1
        "#;

        let now = chrono::Utc::now().timestamp_millis();

        let mut q_params = new_query_params();
        q_params.push(text_param(":id", id));
        q_params.push(integer_param(":now", now));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        let dto: Option<OauthCodeDto> = collect_row(row_result)?;
        Ok(dto)
    }

    pub async fn find_by_code(&self, code: &str) -> Result<Option<OauthCodeDto>> {
        let query = r#"
            SELECT
                id,
                code,
                state,
                redirect_uri,
                scope,
                app_id,
                org_id,
                user_id,
                created_at,
                expires_at
            FROM oauth_codes
            WHERE
                code = :code
                AND expires_at > :now
            LIMIT 1
        "#;

        let now = chrono::Utc::now().timestamp_millis();

        let mut q_params = new_query_params();
        q_params.push(text_param(":code", code.to_string()));
        q_params.push(integer_param(":now", now));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        let dto: Option<OauthCodeDto> = collect_row(row_result)?;
        Ok(dto)
    }

    pub async fn delete(&self, id: String) -> Result<()> {
        let query = r#"
            DELETE FROM oauth_codes
            WHERE
                id = :id
        "#;

        let mut q_params = new_query_params();
        q_params.push(text_param(":id", id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let _ = stmt.execute(q_params).await.context(DbStatementSnafu)?;

        Ok(())
    }

    pub async fn delete_expired(&self) -> Result<()> {
        let query = r#"
            DELETE FROM oauth_codes
            WHERE
                expires_at <= :now
        "#;

        let now = chrono::Utc::now().timestamp_millis();

        let mut q_params = new_query_params();
        q_params.push(integer_param(":now", now));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let _ = stmt.execute(q_params).await.context(DbStatementSnafu)?;

        Ok(())
    }
}
