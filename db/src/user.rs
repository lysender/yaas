use snafu::ResultExt;
use turso::{Connection, Row};

use crate::Result;
use crate::error::{DbPrepareSnafu, DbStatementSnafu, DbTransactionSnafu};
use crate::turso_decode::{
    FromTursoRow, collect_count, collect_row, collect_rows, row_integer, row_text,
};
use crate::turso_params::{integer_param, new_query_params, text_param};
use yaas::dto::{ListUsersParamsDto, NewUserDto, NewUserWithPasswordDto, UpdateUserDto, UserDto};
use yaas::pagination::{Paginated, PaginationParams};
use yaas::utils::{IdPrefix, generate_id};

#[derive(Clone)]
pub struct User {
    pub id: String,
    pub email: String,
    pub name: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

impl From<User> for UserDto {
    fn from(user: User) -> Self {
        UserDto {
            id: user.id,
            email: user.email,
            name: user.name,
            status: user.status,
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

impl FromTursoRow for UserDto {
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row_text(row, 0)?,
            email: row_text(row, 1)?,
            name: row_text(row, 2)?,
            status: row_text(row, 3)?,
            created_at: row_integer(row, 4)?,
            updated_at: row_integer(row, 5)?,
        })
    }
}

pub struct UserRepo {
    db_pool: Connection,
}

impl UserRepo {
    pub fn new(db_pool: Connection) -> Self {
        Self { db_pool }
    }

    async fn listing_count(&self, params: ListUsersParamsDto) -> Result<i64> {
        let mut query = r#"
            SELECT COUNT(*) AS total_count
            FROM users
            WHERE
                deleted_at IS NULL
        "#
        .to_string();

        let mut q_params = new_query_params();

        if let Some(keyword) = params.keyword
            && !keyword.is_empty()
        {
            query.push_str(" AND (email LIKE :keyword OR name LIKE :keyword)");
            let pattern = format!("%{}%", keyword);
            q_params.push(text_param(":keyword", pattern));
        }

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        collect_count(row_result)
    }

    pub async fn list(&self, params: ListUsersParamsDto) -> Result<Paginated<UserDto>> {
        let mut query = r#"
            SELECT
                id,
                email,
                name,
                status,
                created_at,
                updated_at
            FROM users
            WHERE
                deleted_at IS NULL
        "#
        .to_string();

        let mut q_params = new_query_params();
        let count_params = params.clone();

        if let Some(keyword) = params.keyword
            && !keyword.is_empty()
        {
            query.push_str(" AND (email LIKE :keyword OR name LIKE :keyword)");
            let pattern = format!("%{}%", keyword);
            q_params.push(text_param(":keyword", pattern));
        }

        let total_records = self.listing_count(count_params).await?;
        let pagination = PaginationParams::new(total_records, params.page, params.per_page, None);

        if pagination.total_pages == 0 {
            return Ok(Paginated::new(
                Vec::new(),
                pagination.page,
                pagination.per_page,
                pagination.total_records,
            ));
        }

        query.push_str(" ORDER BY email ASC LIMIT :limit OFFSET :offset");
        q_params.push(integer_param(":limit", pagination.per_page as i64));
        q_params.push(integer_param(":offset", pagination.offset));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let mut rows = stmt.query(q_params).await.context(DbStatementSnafu)?;
        let items: Vec<UserDto> = collect_rows(&mut rows).await?;

        Ok(Paginated::new(
            items,
            pagination.page,
            pagination.per_page,
            pagination.total_records,
        ))
    }

    pub async fn create(&self, data: NewUserDto) -> Result<UserDto> {
        let query = r#"
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

        let id = generate_id(IdPrefix::User);
        let status = "active".to_string();
        let today = chrono::Utc::now().timestamp_millis();

        let mut q_params = new_query_params();
        q_params.push(text_param(":id", id.clone()));
        q_params.push(text_param(":email", data.email.clone()));
        q_params.push(text_param(":name", data.name.clone()));
        q_params.push(text_param(":status", status.clone()));
        q_params.push(integer_param(":created_at", today));
        q_params.push(integer_param(":updated_at", today));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let affected = stmt.execute(q_params).await.context(DbStatementSnafu)?;
        assert!(affected > 0, "Must insert a new row");

        let user = UserDto {
            id,
            email: data.email,
            name: data.name,
            status,
            created_at: today,
            updated_at: today,
        };

        Ok(user)
    }

    pub async fn create_with_password(&self, new_user: NewUserWithPasswordDto) -> Result<UserDto> {
        let user_id = generate_id(IdPrefix::User);
        let status = "active".to_string();
        let today = chrono::Utc::now().timestamp_millis();

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
        user_params.push(text_param(":email", new_user.email.clone()));
        user_params.push(text_param(":name", new_user.name.clone()));
        user_params.push(text_param(":status", status.clone()));
        user_params.push(integer_param(":created_at", today));
        user_params.push(integer_param(":updated_at", today));

        let mut conn = self.db_pool.clone();
        let tx = conn.transaction().await.context(DbTransactionSnafu)?;

        let mut user_stmt = tx.prepare(user_query).await.context(DbPrepareSnafu)?;

        let user_affected = user_stmt
            .execute(user_params)
            .await
            .context(DbStatementSnafu)?;

        assert!(user_affected > 0, "Must insert a new user row");

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
        password_params.push(text_param(":password", new_user.password));
        password_params.push(integer_param(":created_at", today));
        password_params.push(integer_param(":updated_at", today));

        let mut password_stmt = tx.prepare(passwd_query).await.context(DbPrepareSnafu)?;

        let password_affected = password_stmt
            .execute(password_params)
            .await
            .context(DbStatementSnafu)?;

        assert!(password_affected > 0, "Must insert a new password row");

        tx.commit().await.context(DbTransactionSnafu)?;

        Ok(UserDto {
            id: user_id,
            email: new_user.email,
            name: new_user.name,
            status,
            created_at: today,
            updated_at: today,
        })
    }

    pub async fn get(&self, id: String) -> Result<Option<UserDto>> {
        let query = r#"
            SELECT
                id,
                email,
                name,
                status,
                created_at,
                updated_at
            FROM users
            WHERE
                deleted_at IS NULL
                AND id = :id
            LIMIT 1
        "#;

        let mut q_params = new_query_params();
        q_params.push(text_param(":id", id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        let dto: Option<UserDto> = collect_row(row_result)?;
        Ok(dto)
    }

    pub async fn find_by_email(&self, email: String) -> Result<Option<UserDto>> {
        let query = r#"
            SELECT
                id,
                email,
                name,
                status,
                created_at,
                updated_at
            FROM users
            WHERE
                deleted_at IS NULL
                AND email = :email
            LIMIT 1
        "#;

        let mut q_params = new_query_params();
        q_params.push(text_param(":email", email));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        let dto: Option<UserDto> = collect_row(row_result)?;
        Ok(dto)
    }

    pub async fn update(&self, id: String, data: UpdateUserDto) -> Result<bool> {
        if data.status.is_none() && data.name.is_none() {
            return Ok(false);
        }

        let mut query = "UPDATE users SET ".to_string();
        let mut set_parts: Vec<&str> = Vec::new();
        let mut q_params = new_query_params();

        if let Some(name) = data.name {
            set_parts.push("name = :name");
            q_params.push(text_param(":name", name));
        }

        if let Some(status) = data.status {
            set_parts.push("status = :status");
            q_params.push(text_param(":status", status));
        }

        let updated_at = chrono::Utc::now().timestamp_millis();
        set_parts.push("updated_at = :updated_at");
        q_params.push(integer_param(":updated_at", updated_at));

        query.push_str(&set_parts.join(", "));
        query.push_str(" WHERE id = :id AND deleted_at IS NULL");
        q_params.push(text_param(":id", id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let affected = stmt.execute(q_params).await.context(DbStatementSnafu)?;
        Ok(affected > 0)
    }

    pub async fn delete(&self, id: String) -> Result<bool> {
        let query = r#"
            UPDATE users
            SET
                deleted_at = :deleted_at
            WHERE
                id = :id
                AND deleted_at IS NULL
        "#;

        let deleted_at = chrono::Utc::now().timestamp_millis();
        let mut q_params = new_query_params();
        q_params.push(integer_param(":deleted_at", deleted_at));
        q_params.push(text_param(":id", id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let affected = stmt.execute(q_params).await.context(DbStatementSnafu)?;
        Ok(affected > 0)
    }
}
