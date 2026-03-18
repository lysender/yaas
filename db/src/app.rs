use snafu::ResultExt;
use turso::{Connection, Row};

use crate::Result;
use crate::error::{DbPrepareSnafu, DbStatementSnafu};
use crate::turso_decode::{
    FromTursoRow, collect_count, collect_row, collect_rows, row_integer, row_text,
};
use crate::turso_params::{integer_param, new_query_params, text_param};
use yaas::dto::{AppDto, ListAppsParamsDto, NewAppDto, UpdateAppDto};
use yaas::pagination::{Paginated, PaginationParams};
use yaas::utils::{IdPrefix, generate_id};

pub struct App {
    pub id: String,
    pub name: String,
    pub client_id: String,
    pub client_secret: String,
    pub redirect_uri: String,
    pub created_at: i64,
    pub updated_at: i64,
    pub deleted_at: Option<i64>,
}

impl From<App> for AppDto {
    fn from(app: App) -> Self {
        AppDto {
            id: app.id,
            name: app.name,
            client_id: app.client_id,
            client_secret: app.client_secret,
            redirect_uri: app.redirect_uri,
            created_at: app.created_at,
            updated_at: app.updated_at,
        }
    }
}

impl FromTursoRow for AppDto {
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row_text(row, 0)?,
            name: row_text(row, 1)?,
            client_id: row_text(row, 2)?,
            client_secret: row_text(row, 3)?,
            redirect_uri: row_text(row, 4)?,
            created_at: row_integer(row, 5)?,
            updated_at: row_integer(row, 6)?,
        })
    }
}

pub struct AppRepo {
    db_pool: Connection,
}

impl AppRepo {
    pub fn new(db_pool: Connection) -> Self {
        Self { db_pool }
    }

    async fn listing_count(&self, params: ListAppsParamsDto) -> Result<i64> {
        let mut query = r#"
            SELECT COUNT(*) AS total_count
            FROM apps
            WHERE
                deleted_at IS NULL
        "#
        .to_string();

        let mut q_params = new_query_params();

        if let Some(keyword) = params.keyword
            && !keyword.is_empty()
        {
            query.push_str(" AND name LIKE :keyword");
            let pattern = format!("%{}%", keyword);
            q_params.push(text_param(":keyword", pattern));
        }

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        collect_count(row_result)
    }

    pub async fn list(&self, params: ListAppsParamsDto) -> Result<Paginated<AppDto>> {
        let mut query = r#"
            SELECT
                id,
                name,
                client_id,
                client_secret,
                redirect_uri,
                created_at,
                updated_at
            FROM apps
            WHERE
                deleted_at IS NULL
        "#
        .to_string();

        let mut q_params = new_query_params();
        let count_params = params.clone();

        if let Some(keyword) = params.keyword
            && !keyword.is_empty()
        {
            query.push_str(" AND name LIKE :keyword");
            let pattern = format!("%{}%", keyword);
            q_params.push(text_param(":keyword", pattern));
        }

        let total_records = self.listing_count(count_params).await?;

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

        query.push_str(" ORDER BY name ASC LIMIT :limit OFFSET :offset");

        q_params.push(integer_param(":limit", pagination.per_page as i64));
        q_params.push(integer_param(":offset", pagination.offset));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let mut rows = stmt.query(q_params).await.context(DbStatementSnafu)?;
        let items: Vec<AppDto> = collect_rows(&mut rows).await?;

        Ok(Paginated::new(
            items,
            pagination.page,
            pagination.per_page,
            pagination.total_records,
        ))
    }

    pub async fn create(&self, data: NewAppDto) -> Result<AppDto> {
        let query = r#"
            INSERT INTO apps
            (
                id,
                name,
                client_id,
                client_secret,
                redirect_uri,
                created_at,
                updated_at,
                deleted_at
            )
            VALUES
            (
                :id,
                :name,
                :client_id,
                :client_secret,
                :redirect_uri,
                :created_at,
                :updated_at,
                NULL
            )
        "#;

        let id = generate_id(IdPrefix::App);
        let today = chrono::Utc::now().timestamp_millis();
        let client_id = generate_id(IdPrefix::ClientId);
        let client_secret = generate_id(IdPrefix::ClientSecret);

        let mut q_params = new_query_params();

        q_params.push(text_param(":name", data.name.clone()));
        q_params.push(text_param(":id", id.clone()));
        q_params.push(text_param(":client_id", client_id.clone()));
        q_params.push(text_param(":client_secret", client_secret.clone()));
        q_params.push(text_param(":redirect_uri", data.redirect_uri.clone()));
        q_params.push(integer_param(":created_at", today));
        q_params.push(integer_param(":updated_at", today));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let affected = stmt.execute(q_params).await.context(DbStatementSnafu)?;
        assert!(affected > 0, "Must insert a new row");

        let app = App {
            id,
            name: data.name,
            client_id: client_id,
            client_secret: client_secret,
            redirect_uri: data.redirect_uri,
            created_at: today,
            updated_at: today,
            deleted_at: None,
        };

        Ok(app.into())
    }

    pub async fn get(&self, id: String) -> Result<Option<AppDto>> {
        let query = r#"
            SELECT
                id,
                name,
                client_id,
                client_secret,
                redirect_uri,
                created_at,
                updated_at
            FROM apps
            WHERE
                deleted_at IS NULL
                AND id = :id
            LIMIT 1
        "#;

        let mut q_params = new_query_params();
        q_params.push(text_param(":id", id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        let dto: Option<AppDto> = collect_row(row_result)?;
        Ok(dto)
    }

    pub async fn find_by_client_id(&self, client_id: String) -> Result<Option<AppDto>> {
        let query = r#"
            SELECT
                id,
                name,
                client_id,
                client_secret,
                redirect_uri,
                created_at,
                updated_at
            FROM apps
            WHERE
                deleted_at IS NULL
                AND client_id = :client_id
            LIMIT 1
        "#;

        let mut q_params = new_query_params();
        q_params.push(text_param(":client_id", client_id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        let dto: Option<AppDto> = collect_row(row_result)?;
        Ok(dto)
    }

    pub async fn update(&self, id: String, data: UpdateAppDto) -> Result<bool> {
        // Do not allow empty update
        if data.name.is_none() && data.redirect_uri.is_none() {
            return Ok(false);
        }

        let mut query = "UPDATE apps SET ".to_string();
        let mut set_parts: Vec<&str> = Vec::new();
        let mut q_params = new_query_params();

        if let Some(name) = data.name {
            set_parts.push("name = :name");
            q_params.push(text_param(":name", name));
        }

        if let Some(redirect_uri) = data.redirect_uri {
            set_parts.push("redirect_uri = :redirect_uri");
            q_params.push(text_param(":redirect_uri", redirect_uri));
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

    pub async fn regenerate_secret(&self, id: String) -> Result<bool> {
        let query = r#"
            UPDATE apps
            SET
                client_id = :client_id,
                client_secret = :client_secret,
                updated_at = :updated_at
            WHERE
                id = :id
                AND deleted_at IS NULL
        "#;

        let client_id = generate_id(IdPrefix::ClientId);
        let client_secret = generate_id(IdPrefix::ClientSecret);
        let updated_at = chrono::Utc::now().timestamp_millis();

        let mut q_params = new_query_params();
        q_params.push(text_param(":client_id", client_id));
        q_params.push(text_param(":client_secret", client_secret));
        q_params.push(integer_param(":updated_at", updated_at));
        q_params.push(text_param(":id", id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let affected = stmt.execute(q_params).await.context(DbStatementSnafu)?;
        Ok(affected > 0)
    }

    pub async fn delete(&self, id: String) -> Result<bool> {
        let query = r#"
            UPDATE apps
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
