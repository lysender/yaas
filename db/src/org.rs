use snafu::ResultExt;
use turso::{Connection, Row};

use crate::Result;
use crate::error::{DbPrepareSnafu, DbStatementSnafu, DbTransactionSnafu};
use crate::turso_decode::{
    FromTursoRow, collect_count, collect_row, collect_rows, opt_row_text, row_integer, row_text,
};
use crate::turso_params::{integer_param, new_query_params, text_param};
use yaas::dto::{
    ListOrgOwnerSuggestionsParamsDto, ListOrgsParamsDto, NewOrgDto, OrgDto, OrgOwnerSuggestionDto,
    UpdateOrgDto,
};
use yaas::pagination::{Paginated, PaginationParams};
use yaas::utils::{IdPrefix, generate_id};

impl FromTursoRow for OrgDto {
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row_text(row, 0)?,
            name: row_text(row, 1)?,
            status: row_text(row, 2)?,
            owner_id: opt_row_text(row, 3)?,
            owner_email: opt_row_text(row, 4)?,
            owner_name: opt_row_text(row, 5)?,
            created_at: row_integer(row, 6)?,
            updated_at: row_integer(row, 7)?,
        })
    }
}

impl FromTursoRow for OrgOwnerSuggestionDto {
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row_text(row, 0)?,
            email: row_text(row, 1)?,
            name: row_text(row, 2)?,
        })
    }
}

pub struct OrgRepo {
    db_pool: Connection,
}

impl OrgRepo {
    pub fn new(db_pool: Connection) -> Self {
        Self { db_pool }
    }

    async fn listing_count(&self, params: ListOrgsParamsDto) -> Result<i64> {
        let mut query = r#"
            SELECT COUNT(*) AS total_count
            FROM orgs
            LEFT JOIN users ON users.id = orgs.owner_id
            WHERE
                orgs.deleted_at IS NULL
        "#
        .to_string();

        let mut q_params = new_query_params();

        if let Some(keyword) = params.keyword
            && !keyword.is_empty()
        {
            query.push_str(" AND (orgs.name LIKE :keyword OR users.email LIKE :keyword)");
            let pattern = format!("%{}%", keyword);
            q_params.push(text_param(":keyword", pattern));
        }

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        collect_count(row_result)
    }

    pub async fn list(&self, params: ListOrgsParamsDto) -> Result<Paginated<OrgDto>> {
        let mut query = r#"
            SELECT
                orgs.id,
                orgs.name,
                orgs.status,
                orgs.owner_id,
                users.email AS owner_email,
                users.name AS owner_name,
                orgs.created_at,
                orgs.updated_at
            FROM orgs
            LEFT JOIN users ON users.id = orgs.owner_id
            WHERE
                orgs.deleted_at IS NULL
        "#
        .to_string();

        let mut q_params = new_query_params();

        if let Some(keyword) = params.keyword.clone()
            && !keyword.is_empty()
        {
            query.push_str(" AND (orgs.name LIKE :keyword OR users.email LIKE :keyword)");
            let pattern = format!("%{}%", keyword);
            q_params.push(text_param(":keyword", pattern));
        }

        let total_records = self.listing_count(params.clone()).await?;
        let pagination = PaginationParams::new(total_records, params.page, params.per_page, None);

        if pagination.total_pages == 0 {
            return Ok(Paginated::new(
                Vec::new(),
                pagination.page,
                pagination.per_page,
                pagination.total_records,
            ));
        }

        query.push_str(" ORDER BY orgs.name ASC LIMIT :limit OFFSET :offset");
        q_params.push(integer_param(":limit", pagination.per_page as i64));
        q_params.push(integer_param(":offset", pagination.offset));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let mut rows = stmt.query(q_params).await.context(DbStatementSnafu)?;
        let items: Vec<OrgDto> = collect_rows(&mut rows).await?;

        Ok(Paginated::new(
            items,
            pagination.page,
            pagination.per_page,
            pagination.total_records,
        ))
    }

    async fn list_owner_suggestions_count(
        &self,
        params: ListOrgOwnerSuggestionsParamsDto,
    ) -> Result<i64> {
        let mut query = r#"
            SELECT COUNT(*) AS total_count
            FROM users
            LEFT JOIN superusers ON superusers.id = users.id
            WHERE
                superusers.id IS NULL
                AND users.deleted_at IS NULL
        "#
        .to_string();

        let mut q_params = new_query_params();

        if let Some(keyword) = params.keyword
            && !keyword.is_empty() {
                query.push_str(" AND (users.name LIKE :keyword OR users.email LIKE :keyword)");
                let pattern = format!("%{}%", keyword);
                q_params.push(text_param(":keyword", pattern));
            }

        if let Some(exclude_user_id) = params.exclude_id {
            query.push_str(" AND users.id <> :exclude_id");
            q_params.push(text_param(":exclude_id", exclude_user_id.to_string()));
        }

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        collect_count(row_result)
    }

    pub async fn list_owner_suggestions(
        &self,
        params: ListOrgOwnerSuggestionsParamsDto,
    ) -> Result<Paginated<OrgOwnerSuggestionDto>> {
        let mut query = r#"
            SELECT
                users.id,
                users.email,
                users.name
            FROM users
            LEFT JOIN superusers ON superusers.id = users.id
            WHERE
                superusers.id IS NULL
                AND users.deleted_at IS NULL
        "#
        .to_string();

        let mut q_params = new_query_params();

        if let Some(keyword) = params.keyword.clone()
            && !keyword.is_empty() {
                query.push_str(" AND (users.name LIKE :keyword OR users.email LIKE :keyword)");
                let pattern = format!("%{}%", keyword);
                q_params.push(text_param(":keyword", pattern));
            }

        if let Some(exclude_user_id) = params.exclude_id.as_ref() {
            query.push_str(" AND users.id <> :exclude_id");
            q_params.push(text_param(":exclude_id", exclude_user_id.to_string()));
        }

        let total_records = self.list_owner_suggestions_count(params.clone()).await?;
        let pagination = PaginationParams::new(total_records, params.page, params.per_page, None);

        if pagination.total_pages == 0 {
            return Ok(Paginated::new(
                Vec::new(),
                pagination.page,
                pagination.per_page,
                pagination.total_records,
            ));
        }

        query.push_str(" ORDER BY users.email ASC LIMIT :limit OFFSET :offset");
        q_params.push(integer_param(":limit", pagination.per_page as i64));
        q_params.push(integer_param(":offset", pagination.offset));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let mut rows = stmt.query(q_params).await.context(DbStatementSnafu)?;
        let items: Vec<OrgOwnerSuggestionDto> = collect_rows(&mut rows).await?;

        Ok(Paginated::new(
            items,
            pagination.page,
            pagination.per_page,
            pagination.total_records,
        ))
    }

    pub async fn create(&self, data: NewOrgDto) -> Result<OrgDto> {
        let org_id = generate_id(IdPrefix::Org);
        let member_id = generate_id(IdPrefix::OrgMember);
        let today = chrono::Utc::now().timestamp_millis();

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
        org_params.push(text_param(":name", data.name.clone()));
        org_params.push(text_param(":status", "active".to_string()));
        org_params.push(text_param(":owner_id", data.owner_id.clone()));
        org_params.push(integer_param(":created_at", today));
        org_params.push(integer_param(":updated_at", today));

        let member_query = r#"
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

        let mut member_params = new_query_params();
        member_params.push(text_param(":id", member_id));
        member_params.push(text_param(":org_id", org_id.clone()));
        member_params.push(text_param(":user_id", data.owner_id.clone()));
        member_params.push(text_param(":roles", "OrgAdmin".to_string()));
        member_params.push(text_param(":status", "active".to_string()));
        member_params.push(integer_param(":created_at", today));
        member_params.push(integer_param(":updated_at", today));

        let mut conn = self.db_pool.clone();
        let tx = conn.transaction().await.context(DbTransactionSnafu)?;

        let mut org_stmt = tx.prepare(org_query).await.context(DbPrepareSnafu)?;
        let org_affected = org_stmt
            .execute(org_params)
            .await
            .context(DbStatementSnafu)?;
        assert!(org_affected > 0, "Must insert a new org row");

        let mut member_stmt = tx.prepare(member_query).await.context(DbPrepareSnafu)?;
        let member_affected = member_stmt
            .execute(member_params)
            .await
            .context(DbStatementSnafu)?;
        assert!(member_affected > 0, "Must insert a new org member row");

        tx.commit().await.context(DbTransactionSnafu)?;

        Ok(OrgDto {
            id: org_id,
            name: data.name,
            status: "active".to_string(),
            owner_id: Some(data.owner_id),
            owner_email: None,
            owner_name: None,
            created_at: today,
            updated_at: today,
        })
    }

    pub async fn get(&self, id: String) -> Result<Option<OrgDto>> {
        let query = r#"
            SELECT
                orgs.id,
                orgs.name,
                orgs.status,
                orgs.owner_id,
                users.email AS owner_email,
                users.name AS owner_name,
                orgs.created_at,
                orgs.updated_at
            FROM orgs
            LEFT JOIN users ON users.id = orgs.owner_id
            WHERE
                orgs.id = :id
                AND orgs.deleted_at IS NULL
            LIMIT 1
        "#;

        let mut q_params = new_query_params();
        q_params.push(text_param(":id", id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        let dto: Option<OrgDto> = collect_row(row_result)?;
        Ok(dto)
    }

    pub async fn update(&self, id: String, data: UpdateOrgDto) -> Result<bool> {
        if data.status.is_none() && data.name.is_none() && data.owner_id.is_none() {
            return Ok(false);
        }

        let mut query = "UPDATE orgs SET ".to_string();
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

        if let Some(owner_id) = data.owner_id {
            set_parts.push("owner_id = :owner_id");
            q_params.push(text_param(":owner_id", owner_id));
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
            UPDATE orgs
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

    pub async fn test_read(&self) -> Result<()> {
        let query = r#"
            SELECT
                orgs.id,
                orgs.name,
                orgs.status,
                orgs.owner_id,
                users.email AS owner_email,
                users.name AS owner_name,
                orgs.created_at,
                orgs.updated_at
            FROM orgs
            LEFT JOIN users ON users.id = orgs.owner_id
            WHERE
                orgs.deleted_at IS NULL
            LIMIT 1
        "#;

        let q_params = new_query_params();
        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        let _dto: Option<OrgDto> = collect_row(row_result)?;
        Ok(())
    }
}
