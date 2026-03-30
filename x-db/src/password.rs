use snafu::ResultExt;
use turso::{Connection, Row};

use crate::Result;
use crate::error::{DbPrepareSnafu, DbStatementSnafu};
use crate::turso_decode::{FromTursoRow, collect_row, row_integer, row_text};
use crate::turso_params::{integer_param, new_query_params, text_param};
use yaas::dto::{NewPasswordDto, PasswordDto};

impl FromTursoRow for PasswordDto {
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row_text(row, 0)?,
            password: row_text(row, 1)?,
            created_at: row_integer(row, 2)?,
            updated_at: row_integer(row, 3)?,
        })
    }
}

pub struct PasswordRepo {
    db_pool: Connection,
}

impl PasswordRepo {
    pub fn new(db_pool: Connection) -> Self {
        Self { db_pool }
    }

    pub async fn create(&self, user_id: String, data: NewPasswordDto) -> Result<()> {
        let query = r#"
            INSERT INTO passwords
            (
                id,
                password,
                created_at,
                updated_at
            )
            VALUES
            (
                :id,
                :password,
                :created_at,
                :updated_at
            )
        "#;

        let today = chrono::Utc::now().timestamp_millis();
        let mut q_params = new_query_params();
        q_params.push(text_param(":id", user_id));
        q_params.push(text_param(":password", data.password));
        q_params.push(integer_param(":created_at", today));
        q_params.push(integer_param(":updated_at", today));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let affected = stmt.execute(q_params).await.context(DbStatementSnafu)?;
        assert!(affected > 0, "Must insert a new row");

        Ok(())
    }

    pub async fn get(&self, user_id: String) -> Result<Option<PasswordDto>> {
        let query = r#"
            SELECT
                id,
                password,
                created_at,
                updated_at
            FROM passwords
            WHERE
                id = :id
            LIMIT 1
        "#;

        let mut q_params = new_query_params();
        q_params.push(text_param(":id", user_id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        let dto: Option<PasswordDto> = collect_row(row_result)?;
        Ok(dto)
    }

    pub async fn update(&self, user_id: String, data: NewPasswordDto) -> Result<bool> {
        let query = r#"
            UPDATE passwords
            SET
                password = :password,
                updated_at = :updated_at
            WHERE
                id = :id
        "#;

        let updated_at = chrono::Utc::now().timestamp_millis();

        let mut q_params = new_query_params();
        q_params.push(text_param(":password", data.password));
        q_params.push(integer_param(":updated_at", updated_at));
        q_params.push(text_param(":id", user_id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let affected = stmt.execute(q_params).await.context(DbStatementSnafu)?;

        Ok(affected > 0)
    }

    pub async fn delete(&self, user_id: String) -> Result<()> {
        let query = r#"
            DELETE FROM passwords
            WHERE
                id = :id
        "#;

        let mut q_params = new_query_params();
        q_params.push(text_param(":id", user_id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let _ = stmt.execute(q_params).await.context(DbStatementSnafu)?;

        Ok(())
    }
}
