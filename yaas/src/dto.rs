use serde::{Deserialize, Serialize};

use crate::buffed::{
    ActorBuf, AppBuf, ErrorMessageBuf, OauthCodeBuf, OrgAppBuf, OrgBuf, OrgMemberBuf,
    OrgMembershipBuf, PasswordBuf, SuperuserBuf, UserBuf,
};
use crate::role::{Permission, Role};

#[derive(Clone, Serialize, Deserialize)]
pub struct UserDto {
    pub id: String,
    pub email: String,
    pub name: String,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<UserBuf> for UserDto {
    fn from(user: UserBuf) -> Self {
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

#[derive(Clone, Serialize, Deserialize)]
pub struct SuperuserDto {
    pub id: String,
    pub created_at: String,
}

impl From<SuperuserBuf> for SuperuserDto {
    fn from(su: SuperuserBuf) -> Self {
        SuperuserDto {
            id: su.id,
            created_at: su.created_at,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct PasswordDto {
    pub id: String,
    pub password: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<PasswordBuf> for PasswordDto {
    fn from(pw: PasswordBuf) -> Self {
        PasswordDto {
            id: pw.id,
            password: pw.password,
            created_at: pw.created_at,
            updated_at: pw.updated_at,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ActorDto {
    pub id: String,
    pub org_id: String,
    pub scope: String,
    pub user: UserDto,
    pub roles: Vec<Role>,
    pub permissions: Vec<Permission>,
}

// impl From<ActorBuf> for ActorDto {
//     fn from(actor: ActorBuf) -> Self {
//         ActorDto {
//             id: actor.id,
//             org_id: actor.org_id,
//             scope: actor.scope,
//             user: actor.user.into(),
//             roles: actor.roles.into_iter().map(|r| r.into()).collect(),
//             permissions: actor.permissions,
//         }
//     }
// }

#[derive(Clone, Serialize, Deserialize)]
pub struct OrgDto {
    pub id: String,
    pub name: String,
    pub status: String,
    pub owner_id: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<OrgBuf> for OrgDto {
    fn from(org: OrgBuf) -> Self {
        OrgDto {
            id: org.id,
            name: org.name,
            status: org.status,
            owner_id: org.owner_id,
            created_at: org.created_at,
            updated_at: org.updated_at,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OrgMemberDto {
    pub id: String,
    pub org_id: String,
    pub user_id: String,
    pub roles: Vec<Role>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}

// impl From<OrgMemberBuf> for OrgMemberDto {
//     fn from(member: OrgMemberBuf) -> Self {
//         OrgMemberDto {
//             id: member.id,
//             org_id: member.org_id,
//             user_id: member.user_id,
//             roles: member.roles.into_iter().map(|r| r.into()).collect(),
//             status: member.status,
//             created_at: member.created_at,
//             updated_at: member.updated_at,
//         }
//     }
// }

#[derive(Clone, Serialize, Deserialize)]
pub struct OrgMembershipDto {
    pub org_id: String,
    pub org_name: String,
    pub user_id: String,
    pub roles: Vec<Role>,
}

// impl From<OrgMembershipBuf> for OrgMembershipDto {
//     fn from(membership: OrgMembershipBuf) -> Self {
//         OrgMembershipDto {
//             org_id: membership.org_id,
//             org_name: membership.org_name,
//             user_id: membership.user_id,
//             roles: membership.roles.into_iter().map(|r| r.into()).collect(),
//         }
//     }
// }

#[derive(Clone, Serialize, Deserialize)]
pub struct AppDto {
    pub id: String,
    pub name: String,
    pub secret: String,
    pub redirect_uri: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<AppBuf> for AppDto {
    fn from(app: AppBuf) -> Self {
        AppDto {
            id: app.id,
            name: app.name,
            secret: app.secret,
            redirect_uri: app.redirect_uri,
            created_at: app.created_at,
            updated_at: app.updated_at,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OrgAppDto {
    pub id: String,
    pub org_id: String,
    pub app_id: String,
    pub created_at: String,
}

impl From<OrgAppBuf> for OrgAppDto {
    fn from(org_app: OrgAppBuf) -> Self {
        OrgAppDto {
            id: org_app.id,
            org_id: org_app.org_id,
            app_id: org_app.app_id,
            created_at: org_app.created_at,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct OauthCodeDto {
    pub id: String,
    pub code: String,
    pub state: String,
    pub redirect_uri: String,
    pub scope: String,
    pub app_id: String,
    pub org_id: String,
    pub user_id: String,
    pub created_at: String,
    pub expires_at: String,
}

impl From<OauthCodeBuf> for OauthCodeDto {
    fn from(code: OauthCodeBuf) -> Self {
        OauthCodeDto {
            id: code.id,
            code: code.code,
            state: code.state,
            redirect_uri: code.redirect_uri,
            scope: code.scope,
            app_id: code.app_id,
            org_id: code.org_id,
            user_id: code.user_id,
            created_at: code.created_at,
            expires_at: code.expires_at,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct ErrorMessageDto {
    pub status_code: u16,
    pub message: String,
    pub error: String,
}

impl From<ErrorMessageBuf> for ErrorMessageDto {
    fn from(err: ErrorMessageBuf) -> Self {
        ErrorMessageDto {
            status_code: err.status_code as u16,
            message: err.message,
            error: err.error,
        }
    }
}
