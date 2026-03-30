use snafu::ResultExt;
use turso::{Connection, Row};

use crate::Result;
use crate::db::turso_decode::{FromTursoRow, collect_row, collect_rows, row_integer, row_text};
use crate::db::turso_params::{integer_param, new_query_params, text_param};
use crate::dto::{NewPasswordDto, NewUserDto, SuperuserDto};
use crate::error::{DbPrepareSnafu, DbStatementSnafu, DbTransactionSnafu};
use crate::utils::{IdPrefix, generate_id};

impl FromTursoRow for SuperuserDto {
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row_text(row, 0)?,
            created_at: row_integer(row, 1)?,
        })
    }
}

pub struct SuperuserRepo {
    db_pool: Connection,
}

impl SuperuserRepo {
    pub fn new(db_pool: Connection) -> Self {
        Self { db_pool }
    }

    pub async fn setup(
        &self,
        new_user: NewUserDto,
        new_password: NewPasswordDto,
    ) -> Result<SuperuserDto> {
        let user_id = generate_id(IdPrefix::User);
        let org_id = generate_id(IdPrefix::Org);
        let org_member_id = generate_id(IdPrefix::OrgMember);
        let created_at = chrono::Utc::now().timestamp_millis();

        let user_query = r#"
            INSERT INTO users
            (
                id,
                email,
                name,
                status,
                created_at,
                updated_at,
                deleted_at
            )
            VALUES
            (
                :id,
                :email,
                :name,
                :status,
                :created_at,
                :updated_at,
                NULL
            )
        "#;

        let mut user_params = new_query_params();
        user_params.push(text_param(":id", user_id.clone()));
        user_params.push(text_param(":email", new_user.email));
        user_params.push(text_param(":name", new_user.name));
        user_params.push(text_param(":status", "active".to_string()));
        user_params.push(integer_param(":created_at", created_at));
        user_params.push(integer_param(":updated_at", created_at));

        let passwd_query = r#"
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

        let mut password_params = new_query_params();
        password_params.push(text_param(":id", user_id.clone()));
        password_params.push(text_param(":password", new_password.password));
        password_params.push(integer_param(":created_at", created_at));
        password_params.push(integer_param(":updated_at", created_at));

        let org_query = r#"
            INSERT INTO orgs
            (
                id,
                name,
                status,
                owner_id,
                created_at,
                updated_at,
                deleted_at
            )
            VALUES
            (
                :id,
                :name,
                :status,
                :owner_id,
                :created_at,
                :updated_at,
                NULL
            )
        "#;

        let mut org_params = new_query_params();
        org_params.push(text_param(":id", org_id.clone()));
        org_params.push(text_param(":name", "Superuser".to_string()));
        org_params.push(text_param(":status", "active".to_string()));
        org_params.push(text_param(":owner_id", user_id.clone()));
        org_params.push(integer_param(":created_at", created_at));
        org_params.push(integer_param(":updated_at", created_at));

        let org_member_query = r#"
            INSERT INTO org_members
            (
                id,
                org_id,
                user_id,
                roles,
                status,
                created_at,
                updated_at
            )
            VALUES
            (
                :id,
                :org_id,
                :user_id,
                :roles,
                :status,
                :created_at,
                :updated_at
            )
        "#;

        let mut org_member_params = new_query_params();
        org_member_params.push(text_param(":id", org_member_id));
        org_member_params.push(text_param(":org_id", org_id));
        org_member_params.push(text_param(":user_id", user_id.clone()));
        org_member_params.push(text_param(":roles", "Superuser".to_string()));
        org_member_params.push(text_param(":status", "active".to_string()));
        org_member_params.push(integer_param(":created_at", created_at));
        org_member_params.push(integer_param(":updated_at", created_at));

        let superuser_query = r#"
            INSERT INTO superusers
            (
                id,
                created_at
            )
            VALUES
            (
                :id,
                :created_at
            )
        "#;

        let mut superuser_params = new_query_params();
        superuser_params.push(text_param(":id", user_id.clone()));
        superuser_params.push(integer_param(":created_at", created_at));

        let mut conn = self.db_pool.clone();
        let tx = conn.transaction().await.context(DbTransactionSnafu)?;

        let mut user_stmt = tx.prepare(user_query).await.context(DbPrepareSnafu)?;
        let user_affected = user_stmt
            .execute(user_params)
            .await
            .context(DbStatementSnafu)?;
        assert!(user_affected > 0, "Must insert a new user row");

        let mut password_stmt = tx.prepare(passwd_query).await.context(DbPrepareSnafu)?;
        let password_affected = password_stmt
            .execute(password_params)
            .await
            .context(DbStatementSnafu)?;
        assert!(password_affected > 0, "Must insert a new password row");

        let mut org_stmt = tx.prepare(org_query).await.context(DbPrepareSnafu)?;
        let org_affected = org_stmt
            .execute(org_params)
            .await
            .context(DbStatementSnafu)?;
        assert!(org_affected > 0, "Must insert a new org row");

        let mut org_member_stmt = tx.prepare(org_member_query).await.context(DbPrepareSnafu)?;
        let org_member_affected = org_member_stmt
            .execute(org_member_params)
            .await
            .context(DbStatementSnafu)?;
        assert!(org_member_affected > 0, "Must insert a new org member row");

        let mut superuser_stmt = tx.prepare(superuser_query).await.context(DbPrepareSnafu)?;
        let superuser_affected = superuser_stmt
            .execute(superuser_params)
            .await
            .context(DbStatementSnafu)?;
        assert!(superuser_affected > 0, "Must insert a new superuser row");

        tx.commit().await.context(DbTransactionSnafu)?;

        Ok(SuperuserDto {
            id: user_id,
            created_at,
        })
    }

    pub async fn list(&self) -> Result<Vec<SuperuserDto>> {
        let query = r#"
            SELECT
                id,
                created_at
            FROM superusers
            ORDER BY created_at ASC
        "#;

        let q_params = new_query_params();

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let mut rows = stmt.query(q_params).await.context(DbStatementSnafu)?;
        let items: Vec<SuperuserDto> = collect_rows(&mut rows).await?;

        Ok(items)
    }

    pub async fn create(&self, user_id: String) -> Result<SuperuserDto> {
        let query = r#"
            INSERT INTO superusers
            (
                id,
                created_at
            )
            VALUES
            (
                :id,
                :created_at
            )
        "#;

        let created_at = chrono::Utc::now().timestamp_millis();

        let mut q_params = new_query_params();
        q_params.push(text_param(":id", user_id.clone()));
        q_params.push(integer_param(":created_at", created_at));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let affected = stmt.execute(q_params).await.context(DbStatementSnafu)?;
        assert!(affected > 0, "Must insert a new superuser row");

        Ok(SuperuserDto {
            id: user_id,
            created_at,
        })
    }

    pub async fn get(&self, id: String) -> Result<Option<SuperuserDto>> {
        let query = r#"
            SELECT
                id,
                created_at
            FROM superusers
            WHERE
                id = :id
            LIMIT 1
        "#;

        let mut q_params = new_query_params();
        q_params.push(text_param(":id", id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        let dto: Option<SuperuserDto> = collect_row(row_result)?;
        Ok(dto)
    }
}
