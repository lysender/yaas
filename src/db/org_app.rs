use snafu::ResultExt;
use turso::{Connection, Row};

use crate::Result;
use crate::db::turso_decode::{
    FromTursoRow, collect_count, collect_row, collect_rows, opt_row_text, row_integer, row_text,
};
use crate::db::turso_params::{integer_param, new_query_params, text_param};
use crate::dto::{ListOrgAppsParamsDto, NewOrgAppDto, OrgAppDto, OrgAppSuggestionDto};
use crate::dto::{Paginated, PaginationParams};
use crate::error::{DbPrepareSnafu, DbStatementSnafu};
use crate::utils::{IdPrefix, generate_id};

impl FromTursoRow for OrgAppDto {
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row_text(row, 0)?,
            org_id: row_text(row, 1)?,
            app_id: row_text(row, 2)?,
            app_name: opt_row_text(row, 3)?,
            created_at: row_integer(row, 4)?,
        })
    }
}

impl FromTursoRow for OrgAppSuggestionDto {
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row_text(row, 0)?,
            name: row_text(row, 1)?,
        })
    }
}

pub struct OrgAppRepo {
    db_pool: Connection,
}

impl OrgAppRepo {
    pub fn new(db_pool: Connection) -> Self {
        Self { db_pool }
    }

    pub async fn listing_count(&self, org_id: String, params: ListOrgAppsParamsDto) -> Result<i64> {
        let mut query = r#"
            SELECT COUNT(*) AS total_count
            FROM org_apps
            LEFT JOIN apps ON apps.id = org_apps.app_id
            WHERE
                org_apps.org_id = :org_id
                AND apps.deleted_at IS NULL
        "#
        .to_string();

        let mut q_params = new_query_params();
        q_params.push(text_param(":org_id", org_id));

        if let Some(keyword) = params.keyword
            && !keyword.is_empty()
        {
            query.push_str(" AND apps.name LIKE :keyword");
            let pattern = format!("%{}%", keyword);
            q_params.push(text_param(":keyword", pattern));
        }

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        collect_count(row_result)
    }

    pub async fn list(
        &self,
        org_id: String,
        params: ListOrgAppsParamsDto,
    ) -> Result<Paginated<OrgAppDto>> {
        let mut query = r#"
            SELECT
                org_apps.id,
                org_apps.org_id,
                org_apps.app_id,
                apps.name,
                org_apps.created_at
            FROM org_apps
            LEFT JOIN apps ON apps.id = org_apps.app_id
            WHERE
                org_apps.org_id = :org_id
                AND apps.deleted_at IS NULL
        "#
        .to_string();

        let mut q_params = new_query_params();
        q_params.push(text_param(":org_id", org_id.clone()));

        if let Some(keyword) = params.keyword.clone()
            && !keyword.is_empty()
        {
            query.push_str(" AND apps.name LIKE :keyword");
            let pattern = format!("%{}%", keyword);
            q_params.push(text_param(":keyword", pattern));
        }

        let total_records = self.listing_count(org_id, params.clone()).await?;
        let pagination = PaginationParams::new(total_records, params.page, params.per_page, None);

        if pagination.total_pages == 0 {
            return Ok(Paginated::new(
                Vec::new(),
                pagination.page,
                pagination.per_page,
                pagination.total_records,
            ));
        }

        query.push_str(" ORDER BY apps.name ASC LIMIT :limit OFFSET :offset");
        q_params.push(integer_param(":limit", pagination.per_page as i64));
        q_params.push(integer_param(":offset", pagination.offset));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let mut rows = stmt.query(q_params).await.context(DbStatementSnafu)?;
        let items: Vec<OrgAppDto> = collect_rows(&mut rows).await?;

        Ok(Paginated::new(
            items,
            pagination.page,
            pagination.per_page,
            pagination.total_records,
        ))
    }

    async fn list_app_suggestions_count(
        &self,
        org_id: String,
        params: ListOrgAppsParamsDto,
    ) -> Result<i64> {
        let mut query = r#"
            SELECT COUNT(*) AS total_count
            FROM apps
            LEFT JOIN org_apps
                ON org_apps.app_id = apps.id
                AND org_apps.org_id = :org_id
            WHERE
                org_apps.app_id IS NULL
                AND apps.deleted_at IS NULL
        "#
        .to_string();

        let mut q_params = new_query_params();
        q_params.push(text_param(":org_id", org_id));

        if let Some(keyword) = params.keyword
            && !keyword.is_empty()
        {
            query.push_str(" AND apps.name LIKE :keyword");
            let pattern = format!("%{}%", keyword);
            q_params.push(text_param(":keyword", pattern));
        }

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        collect_count(row_result)
    }

    pub async fn list_app_suggestions(
        &self,
        org_id: String,
        params: ListOrgAppsParamsDto,
    ) -> Result<Paginated<OrgAppSuggestionDto>> {
        let mut query = r#"
            SELECT
                apps.id,
                apps.name
            FROM apps
            LEFT JOIN org_apps
                ON org_apps.app_id = apps.id
                AND org_apps.org_id = :org_id
            WHERE
                org_apps.app_id IS NULL
                AND apps.deleted_at IS NULL
        "#
        .to_string();

        let mut q_params = new_query_params();
        q_params.push(text_param(":org_id", org_id.clone()));

        if let Some(keyword) = params.keyword.clone()
            && !keyword.is_empty()
        {
            query.push_str(" AND apps.name LIKE :keyword");
            let pattern = format!("%{}%", keyword);
            q_params.push(text_param(":keyword", pattern));
        }

        let total_records = self
            .list_app_suggestions_count(org_id, params.clone())
            .await?;

        let pagination = PaginationParams::new(total_records, params.page, params.per_page, None);

        if pagination.total_pages == 0 {
            return Ok(Paginated::new(
                Vec::new(),
                pagination.page,
                pagination.per_page,
                pagination.total_records,
            ));
        }

        query.push_str(" ORDER BY apps.name ASC LIMIT :limit OFFSET :offset");
        q_params.push(integer_param(":limit", pagination.per_page as i64));
        q_params.push(integer_param(":offset", pagination.offset));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let mut rows = stmt.query(q_params).await.context(DbStatementSnafu)?;
        let items: Vec<OrgAppSuggestionDto> = collect_rows(&mut rows).await?;

        Ok(Paginated::new(
            items,
            pagination.page,
            pagination.per_page,
            pagination.total_records,
        ))
    }

    pub async fn create(&self, org_id: String, data: NewOrgAppDto) -> Result<OrgAppDto> {
        let query = r#"
            INSERT INTO org_apps
            (
                id,
                org_id,
                app_id,
                created_at
            )
            VALUES
            (
                :id,
                :org_id,
                :app_id,
                :created_at
            )
        "#;

        let id = generate_id(IdPrefix::OrgApp);
        let created_at = chrono::Utc::now().timestamp_millis();

        let mut q_params = new_query_params();
        q_params.push(text_param(":id", id.clone()));
        q_params.push(text_param(":org_id", org_id.clone()));
        q_params.push(text_param(":app_id", data.app_id.clone()));
        q_params.push(integer_param(":created_at", created_at));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let affected = stmt.execute(q_params).await.context(DbStatementSnafu)?;
        assert!(affected > 0, "Must insert a new row");

        Ok(OrgAppDto {
            id,
            org_id,
            app_id: data.app_id,
            app_name: None,
            created_at,
        })
    }

    pub async fn get(&self, id: String) -> Result<Option<OrgAppDto>> {
        let query = r#"
            SELECT
                org_apps.id,
                org_apps.org_id,
                org_apps.app_id,
                apps.name,
                org_apps.created_at
            FROM org_apps
            LEFT JOIN apps ON apps.id = org_apps.app_id
            WHERE
                org_apps.id = :id
                AND apps.deleted_at IS NULL
            LIMIT 1
        "#;

        let mut q_params = new_query_params();
        q_params.push(text_param(":id", id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        let dto: Option<OrgAppDto> = collect_row(row_result)?;
        Ok(dto)
    }

    pub async fn find_app(&self, org_id: String, app_id: String) -> Result<Option<OrgAppDto>> {
        let query = r#"
            SELECT
                org_apps.id,
                org_apps.org_id,
                org_apps.app_id,
                apps.name,
                org_apps.created_at
            FROM org_apps
            LEFT JOIN apps ON apps.id = org_apps.app_id
            WHERE
                org_apps.org_id = :org_id
                AND org_apps.app_id = :app_id
                AND apps.deleted_at IS NULL
            LIMIT 1
        "#;

        let mut q_params = new_query_params();
        q_params.push(text_param(":org_id", org_id));
        q_params.push(text_param(":app_id", app_id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        let dto: Option<OrgAppDto> = collect_row(row_result)?;
        Ok(dto)
    }

    pub async fn delete(&self, id: String) -> Result<()> {
        let query = r#"
            DELETE FROM org_apps
            WHERE
                id = :id
        "#;

        let mut q_params = new_query_params();
        q_params.push(text_param(":id", id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let _ = stmt.execute(q_params).await.context(DbStatementSnafu)?;

        Ok(())
    }
}
