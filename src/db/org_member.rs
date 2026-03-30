use snafu::ResultExt;
use turso::{Connection, Row};

use crate::Result;
use crate::error::{DbPrepareSnafu, DbStatementSnafu};
use crate::turso_decode::{
    FromTursoRow, collect_count, collect_row, collect_rows, opt_row_text, row_integer, row_text,
};
use crate::turso_params::{integer_param, new_query_params, text_param};
use yaas::dto::{
    ListOrgMembersParamsDto, NewOrgMemberDto, OrgMemberDto, OrgMemberSuggestionDto,
    OrgMembershipDto, UpdateOrgMemberDto,
};
use yaas::pagination::{ListingParamsDto, Paginated, PaginationParams};
use yaas::role::{Role, to_roles};
use yaas::utils::{IdPrefix, generate_id};

pub struct OrgMemberWithName {
    pub id: String,
    pub org_id: String,
    pub user_id: String,
    pub member_email: Option<String>,
    pub member_name: Option<String>,
    pub roles: String,
    pub status: String,
    pub created_at: i64,
    pub updated_at: i64,
}

impl TryFrom<OrgMemberWithName> for OrgMemberDto {
    type Error = String;

    fn try_from(member: OrgMemberWithName) -> std::result::Result<Self, Self::Error> {
        let mut roles: Vec<Role> = Vec::new();
        if !member.roles.is_empty() {
            let converted_roles: Vec<String> =
                member.roles.split(',').map(|s| s.to_string()).collect();
            let Ok(converted_roles) = to_roles(&converted_roles) else {
                return Err("Roles should convert back to enum".to_string());
            };
            roles = converted_roles;
        }

        Ok(OrgMemberDto {
            id: member.id,
            org_id: member.org_id,
            user_id: member.user_id,
            member_email: member.member_email,
            member_name: member.member_name,
            roles,
            status: member.status,
            created_at: member.created_at,
            updated_at: member.updated_at,
        })
    }
}

impl FromTursoRow for OrgMemberWithName {
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row_text(row, 0)?,
            org_id: row_text(row, 1)?,
            user_id: row_text(row, 2)?,
            member_email: opt_row_text(row, 3)?,
            member_name: opt_row_text(row, 4)?,
            roles: row_text(row, 5)?,
            status: row_text(row, 6)?,
            created_at: row_integer(row, 7)?,
            updated_at: row_integer(row, 8)?,
        })
    }
}

pub struct OrgMembership {
    pub id: String,
    pub name: String,
    pub user_id: String,
    pub roles: String,
}

impl TryFrom<OrgMembership> for OrgMembershipDto {
    type Error = String;

    fn try_from(membership: OrgMembership) -> std::result::Result<Self, Self::Error> {
        let roles: Vec<String> = membership.roles.split(',').map(|s| s.to_string()).collect();
        let Ok(roles) = to_roles(&roles) else {
            return Err("Roles should convert back to enum".to_string());
        };

        Ok(OrgMembershipDto {
            org_id: membership.id,
            org_name: membership.name,
            user_id: membership.user_id,
            roles,
        })
    }
}

impl FromTursoRow for OrgMembership {
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row_text(row, 0)?,
            name: row_text(row, 1)?,
            user_id: row_text(row, 2)?,
            roles: row_text(row, 3)?,
        })
    }
}

impl FromTursoRow for OrgMemberSuggestionDto {
    fn from_row(row: &Row) -> Result<Self> {
        Ok(Self {
            id: row_text(row, 0)?,
            email: row_text(row, 1)?,
            name: row_text(row, 2)?,
        })
    }
}

pub struct OrgMemberRepo {
    db_pool: Connection,
}

impl OrgMemberRepo {
    pub fn new(db_pool: Connection) -> Self {
        Self { db_pool }
    }

    pub async fn listing_count(
        &self,
        org_id: String,
        params: ListOrgMembersParamsDto,
    ) -> Result<i64> {
        let mut query = r#"
            SELECT COUNT(*) AS total_count
            FROM org_members
            LEFT JOIN users ON users.id = org_members.user_id
            WHERE
                org_members.org_id = :org_id
                AND users.deleted_at IS NULL
        "#
        .to_string();

        let mut q_params = new_query_params();
        q_params.push(text_param(":org_id", org_id));

        if let Some(keyword) = params.keyword
            && !keyword.is_empty()
        {
            query.push_str(" AND (users.name LIKE :keyword OR users.email LIKE :keyword)");
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
        params: ListOrgMembersParamsDto,
    ) -> Result<Paginated<OrgMemberDto>> {
        let mut query = r#"
            SELECT
                org_members.id,
                org_members.org_id,
                org_members.user_id,
                users.email,
                users.name,
                org_members.roles,
                org_members.status,
                org_members.created_at,
                org_members.updated_at
            FROM org_members
            LEFT JOIN users ON users.id = org_members.user_id
            WHERE
                org_members.org_id = :org_id
                AND users.deleted_at IS NULL
        "#
        .to_string();

        let mut q_params = new_query_params();
        q_params.push(text_param(":org_id", org_id.clone()));

        if let Some(keyword) = params.keyword.clone()
            && !keyword.is_empty()
        {
            query.push_str(" AND (users.name LIKE :keyword OR users.email LIKE :keyword)");
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

        query.push_str(" ORDER BY users.email ASC LIMIT :limit OFFSET :offset");
        q_params.push(integer_param(":limit", pagination.per_page as i64));
        q_params.push(integer_param(":offset", pagination.offset));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let mut rows = stmt.query(q_params).await.context(DbStatementSnafu)?;
        let items: Vec<OrgMemberWithName> = collect_rows(&mut rows).await?;

        let items: std::result::Result<Vec<OrgMemberDto>, String> =
            items.into_iter().map(|x| x.try_into()).collect();

        match items {
            Ok(list) => Ok(Paginated::new(
                list,
                pagination.page,
                pagination.per_page,
                pagination.total_records,
            )),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn list_memberships_count(&self, user_id: String) -> Result<i64> {
        let query = r#"
            SELECT COUNT(*) AS total_count
            FROM orgs
            INNER JOIN org_members ON orgs.id = org_members.org_id
            WHERE
                orgs.status = 'active'
                AND orgs.deleted_at IS NULL
                AND org_members.status = 'active'
                AND org_members.user_id = :user_id
        "#;

        let mut q_params = new_query_params();
        q_params.push(text_param(":user_id", user_id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        collect_count(row_result)
    }

    pub async fn list_memberships(
        &self,
        user_id: String,
        params: ListingParamsDto,
    ) -> Result<Paginated<OrgMembershipDto>> {
        let query = r#"
            SELECT
                orgs.id,
                orgs.name,
                org_members.user_id,
                org_members.roles
            FROM orgs
            INNER JOIN org_members ON orgs.id = org_members.org_id
            WHERE
                orgs.status = 'active'
                AND orgs.deleted_at IS NULL
                AND org_members.status = 'active'
                AND org_members.user_id = :user_id
            ORDER BY orgs.name ASC
            LIMIT :limit OFFSET :offset
        "#;

        let total_records = self.list_memberships_count(user_id.clone()).await?;
        let pagination = PaginationParams::new(total_records, params.page, params.per_page, None);

        if pagination.total_pages == 0 {
            return Ok(Paginated::new(
                Vec::new(),
                pagination.page,
                pagination.per_page,
                pagination.total_records,
            ));
        }

        let mut q_params = new_query_params();
        q_params.push(text_param(":user_id", user_id));
        q_params.push(integer_param(":limit", pagination.per_page as i64));
        q_params.push(integer_param(":offset", pagination.offset));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let mut rows = stmt.query(q_params).await.context(DbStatementSnafu)?;
        let items: Vec<OrgMembership> = collect_rows(&mut rows).await?;

        let items: std::result::Result<Vec<OrgMembershipDto>, String> =
            items.into_iter().map(|x| x.try_into()).collect();

        match items {
            Ok(list) => Ok(Paginated::new(
                list,
                pagination.page,
                pagination.per_page,
                pagination.total_records,
            )),
            Err(e) => Err(e.into()),
        }
    }

    pub async fn create(&self, org_id: String, data: NewOrgMemberDto) -> Result<OrgMemberDto> {
        let query = r#"
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

        let id = generate_id(IdPrefix::OrgMember);
        let today = chrono::Utc::now().timestamp_millis();
        let roles_raw = data.roles.join(",");

        let mut q_params = new_query_params();
        q_params.push(text_param(":id", id.clone()));
        q_params.push(text_param(":org_id", org_id.clone()));
        q_params.push(text_param(":user_id", data.user_id.clone()));
        q_params.push(text_param(":roles", roles_raw));
        q_params.push(text_param(":status", data.status.clone()));
        q_params.push(integer_param(":created_at", today));
        q_params.push(integer_param(":updated_at", today));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let affected = stmt.execute(q_params).await.context(DbStatementSnafu)?;
        assert!(affected > 0, "Must insert a new row");

        let Ok(roles) = to_roles(&data.roles) else {
            return Err("Roles should convert back to enum".to_string().into());
        };

        Ok(OrgMemberDto {
            id,
            org_id,
            user_id: data.user_id,
            member_email: None,
            member_name: None,
            roles,
            status: data.status,
            created_at: today,
            updated_at: today,
        })
    }

    pub async fn get(&self, id: String) -> Result<Option<OrgMemberDto>> {
        let query = r#"
            SELECT
                org_members.id,
                org_members.org_id,
                org_members.user_id,
                users.email,
                users.name,
                org_members.roles,
                org_members.status,
                org_members.created_at,
                org_members.updated_at
            FROM org_members
            LEFT JOIN users ON users.id = org_members.user_id
            WHERE
                org_members.id = :id
                AND users.deleted_at IS NULL
            LIMIT 1
        "#;

        let mut q_params = new_query_params();
        q_params.push(text_param(":id", id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        let member: Option<OrgMemberWithName> = collect_row(row_result)?;

        match member {
            Some(m) => match m.try_into() {
                Ok(m) => Ok(Some(m)),
                Err(e) => Err(e.into()),
            },
            None => Ok(None),
        }
    }

    pub async fn find_member(
        &self,
        org_id: String,
        user_id: String,
    ) -> Result<Option<OrgMemberDto>> {
        let query = r#"
            SELECT
                org_members.id,
                org_members.org_id,
                org_members.user_id,
                users.email,
                users.name,
                org_members.roles,
                org_members.status,
                org_members.created_at,
                org_members.updated_at
            FROM org_members
            LEFT JOIN users ON users.id = org_members.user_id
            WHERE
                org_members.org_id = :org_id
                AND org_members.user_id = :user_id
                AND users.deleted_at IS NULL
            LIMIT 1
        "#;

        let mut q_params = new_query_params();
        q_params.push(text_param(":org_id", org_id));
        q_params.push(text_param(":user_id", user_id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        let member: Option<OrgMemberWithName> = collect_row(row_result)?;

        match member {
            Some(m) => match m.try_into() {
                Ok(m) => Ok(Some(m)),
                Err(e) => Err(e.into()),
            },
            None => Ok(None),
        }
    }

    async fn list_member_suggestions_count(
        &self,
        org_id: String,
        params: ListOrgMembersParamsDto,
    ) -> Result<i64> {
        let mut query = r#"
            SELECT COUNT(*) AS total_count
            FROM users
            LEFT JOIN org_members
                ON org_members.user_id = users.id
                AND org_members.org_id = :org_id
            LEFT JOIN superusers ON superusers.id = users.id
            WHERE
                org_members.user_id IS NULL
                AND superusers.id IS NULL
                AND users.deleted_at IS NULL
        "#
        .to_string();

        let mut q_params = new_query_params();
        q_params.push(text_param(":org_id", org_id));

        if let Some(keyword) = params.keyword
            && !keyword.is_empty()
        {
            query.push_str(" AND (users.name LIKE :keyword OR users.email LIKE :keyword)");
            let pattern = format!("%{}%", keyword);
            q_params.push(text_param(":keyword", pattern));
        }

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let row_result = stmt.query_row(q_params).await;
        collect_count(row_result)
    }

    pub async fn list_member_suggestions(
        &self,
        org_id: String,
        params: ListOrgMembersParamsDto,
    ) -> Result<Paginated<OrgMemberSuggestionDto>> {
        let mut query = r#"
            SELECT
                users.id,
                users.email,
                users.name
            FROM users
            LEFT JOIN org_members
                ON org_members.user_id = users.id
                AND org_members.org_id = :org_id
            LEFT JOIN superusers ON superusers.id = users.id
            WHERE
                org_members.user_id IS NULL
                AND superusers.id IS NULL
                AND users.deleted_at IS NULL
        "#
        .to_string();

        let mut q_params = new_query_params();
        q_params.push(text_param(":org_id", org_id.clone()));

        if let Some(keyword) = params.keyword.clone()
            && !keyword.is_empty()
        {
            query.push_str(" AND (users.name LIKE :keyword OR users.email LIKE :keyword)");
            let pattern = format!("%{}%", keyword);
            q_params.push(text_param(":keyword", pattern));
        }

        let total_records = self
            .list_member_suggestions_count(org_id, params.clone())
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

        query.push_str(" ORDER BY users.email ASC LIMIT :limit OFFSET :offset");
        q_params.push(integer_param(":limit", pagination.per_page as i64));
        q_params.push(integer_param(":offset", pagination.offset));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let mut rows = stmt.query(q_params).await.context(DbStatementSnafu)?;
        let items: Vec<OrgMemberSuggestionDto> = collect_rows(&mut rows).await?;

        Ok(Paginated::new(
            items,
            pagination.page,
            pagination.per_page,
            pagination.total_records,
        ))
    }

    pub async fn update(&self, id: String, data: UpdateOrgMemberDto) -> Result<bool> {
        if data.status.is_none() && data.roles.is_none() {
            return Ok(false);
        }

        let mut query = "UPDATE org_members SET ".to_string();
        let mut set_parts: Vec<&str> = Vec::new();
        let mut q_params = new_query_params();

        if let Some(roles) = data.roles {
            set_parts.push("roles = :roles");
            q_params.push(text_param(":roles", roles.join(",")));
        }

        if let Some(status) = data.status {
            set_parts.push("status = :status");
            q_params.push(text_param(":status", status));
        }

        let updated_at = chrono::Utc::now().timestamp_millis();
        set_parts.push("updated_at = :updated_at");
        q_params.push(integer_param(":updated_at", updated_at));

        query.push_str(&set_parts.join(", "));
        query.push_str(" WHERE id = :id");
        q_params.push(text_param(":id", id));

        let mut stmt = self.db_pool.prepare(query).await.context(DbPrepareSnafu)?;
        let affected = stmt.execute(q_params).await.context(DbStatementSnafu)?;
        Ok(affected > 0)
    }

    pub async fn delete(&self, id: String) -> Result<()> {
        let query = r#"
            DELETE FROM org_members
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
