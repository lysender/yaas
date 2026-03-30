use chrono::{DateTime, Utc};

use crate::dto::Role;
use crate::dto::{AppDto, OrgAppDto, OrgDto, OrgMemberDto, UserDto};

fn to_ymd(millis: i64) -> String {
    match DateTime::<Utc>::from_timestamp_millis(millis) {
        Some(datetime) => datetime.format("%Y-%m-%d").to_string(),
        None => String::new(),
    }
}

#[derive(Clone)]
pub struct UserView {
    pub id: String,
    pub email: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<UserDto> for UserView {
    fn from(user: UserDto) -> Self {
        UserView {
            id: user.id,
            email: user.email,
            name: user.name,
            status: user.status,
            created_at: to_ymd(user.created_at),
            updated_at: to_ymd(user.updated_at),
        }
    }
}

#[derive(Clone)]
pub struct AppView {
    pub id: String,
    pub name: String,
    #[allow(dead_code)]
    pub client_id: String,
    #[allow(dead_code)]
    pub client_secret: String,
    #[allow(dead_code)]
    pub redirect_uri: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<AppDto> for AppView {
    fn from(app: AppDto) -> Self {
        AppView {
            id: app.id,
            name: app.name,
            client_id: app.client_id,
            client_secret: app.client_secret,
            redirect_uri: app.redirect_uri,
            created_at: to_ymd(app.created_at),
            updated_at: to_ymd(app.updated_at),
        }
    }
}

#[derive(Clone)]
pub struct OrgView {
    pub id: String,
    pub name: String,
    pub status: String,
    #[allow(dead_code)]
    pub owner_id: Option<String>,
    pub owner_email: Option<String>,
    #[allow(dead_code)]
    pub owner_name: Option<String>,
    pub updated_at: String,
    pub created_at: String,
}

impl From<OrgDto> for OrgView {
    fn from(org: OrgDto) -> Self {
        OrgView {
            id: org.id,
            name: org.name,
            status: org.status,
            owner_id: org.owner_id,
            owner_email: org.owner_email,
            owner_name: org.owner_name,
            updated_at: to_ymd(org.updated_at),
            created_at: to_ymd(org.created_at),
        }
    }
}

#[derive(Clone)]
pub struct OrgMemberView {
    #[allow(dead_code)]
    pub id: String,
    pub org_id: String,
    pub user_id: String,
    pub member_email: Option<String>,
    #[allow(dead_code)]
    pub member_name: Option<String>,
    pub roles: Vec<Role>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<OrgMemberDto> for OrgMemberView {
    fn from(member: OrgMemberDto) -> Self {
        OrgMemberView {
            id: member.id,
            org_id: member.org_id,
            user_id: member.user_id,
            member_email: member.member_email,
            member_name: member.member_name,
            roles: member.roles,
            status: member.status,
            created_at: to_ymd(member.created_at),
            updated_at: to_ymd(member.updated_at),
        }
    }
}

#[derive(Clone)]
pub struct OrgAppView {
    #[allow(dead_code)]
    pub id: String,
    pub org_id: String,
    pub app_id: String,
    pub app_name: Option<String>,
    pub created_at: String,
}

impl From<OrgAppDto> for OrgAppView {
    fn from(org_app: OrgAppDto) -> Self {
        OrgAppView {
            id: org_app.id,
            org_id: org_app.org_id,
            app_id: org_app.app_id,
            app_name: org_app.app_name,
            created_at: to_ymd(org_app.created_at),
        }
    }
}
