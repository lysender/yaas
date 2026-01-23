use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::buffed::actor::{ActorBuf, AuthResponseBuf, CredentialsBuf, SwitchAuthContextBuf};
use crate::dto::UserDto;
use crate::role::{
    buffed_to_permissions, buffed_to_roles, buffed_to_scopes, roles_permissions, to_permissions,
    Permission, Role, Scope,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct ActorDto {
    pub id: i32,
    pub org_id: i32,
    pub org_count: i32,
    pub scopes: Vec<Scope>,
    pub user: UserDto,
    pub roles: Vec<Role>,
    pub permissions: Vec<Permission>,
}

impl TryFrom<ActorBuf> for ActorDto {
    type Error = String;

    fn try_from(actor: ActorBuf) -> std::result::Result<Self, Self::Error> {
        let Ok(roles) = buffed_to_roles(&actor.roles) else {
            return Err("Actor roles should convert back to enum".to_string());
        };
        let Ok(permissions) = buffed_to_permissions(&actor.permissions) else {
            return Err("Actor permissions should convert back to enum".to_string());
        };
        let Ok(scopes) = buffed_to_scopes(&actor.scopes) else {
            return Err("Actor scopes should convert back to enum".to_string());
        };

        let Some(user) = actor.user else {
            return Err("Actor user should be present".to_string());
        };

        Ok(ActorDto {
            id: actor.id,
            org_id: actor.org_id,
            org_count: actor.org_count,
            scopes,
            user: user.into(),
            roles,
            permissions,
        })
    }
}

#[derive(Clone)]
pub struct ActorPayloadDto {
    pub id: i32,
    pub org_id: i32,
    pub org_count: i32,
    pub roles: Vec<Role>,
    pub scopes: Vec<Scope>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Actor {
    pub actor: Option<ActorDto>,
}

impl Actor {
    pub fn new(payload: ActorPayloadDto, user: UserDto) -> Self {
        let permissions: Vec<Permission> = roles_permissions(&payload.roles).into_iter().collect();

        // Convert to string to allow sorting
        let mut permissions: Vec<String> = permissions.iter().map(|p| p.to_string()).collect();
        permissions.sort();

        // Convert again to Permission enum
        let permissions: Vec<Permission> =
            to_permissions(&permissions).expect("Permissions should convert back to enum");

        Actor {
            actor: Some(ActorDto {
                id: payload.id,
                org_id: payload.org_id,
                org_count: payload.org_count,
                scopes: payload.scopes,
                user,
                roles: payload.roles,
                permissions,
            }),
        }
    }

    /// Empty actor for unauthenticated requests
    pub fn default() -> Self {
        Actor { actor: None }
    }

    pub fn has_auth_scope(&self) -> bool {
        self.has_scope(Scope::Auth)
    }

    pub fn has_vault_scope(&self) -> bool {
        self.has_scope(Scope::Vault)
    }

    pub fn has_scope(&self, scope: Scope) -> bool {
        match &self.actor {
            Some(actor) => actor.scopes.contains(&scope),
            None => false,
        }
    }

    pub fn has_permissions(&self, permissions: &[Permission]) -> bool {
        match &self.actor {
            Some(actor) => permissions
                .iter()
                .all(|permission| actor.permissions.contains(permission)),
            None => false,
        }
    }

    pub fn is_system_admin(&self) -> bool {
        match &self.actor {
            Some(actor) => actor.roles.contains(&Role::Superuser),
            None => false,
        }
    }

    pub fn member_of(&self, org_id: i32) -> bool {
        match &self.actor {
            Some(actor) => actor.org_id == org_id,
            None => false,
        }
    }
}

#[derive(Deserialize, Serialize, Validate)]
pub struct CredentialsDto {
    #[validate(length(max = 100))]
    #[validate(email)]
    pub email: String,

    #[validate(length(min = 8, max = 60))]
    pub password: String,
}

impl From<CredentialsBuf> for CredentialsDto {
    fn from(creds: CredentialsBuf) -> Self {
        CredentialsDto {
            email: creds.email,
            password: creds.password,
        }
    }
}

#[derive(Deserialize, Serialize, Validate)]
pub struct SwitchAuthContextDto {
    pub org_id: i32,
}

impl From<SwitchAuthContextBuf> for SwitchAuthContextDto {
    fn from(data: SwitchAuthContextBuf) -> Self {
        SwitchAuthContextDto {
            org_id: data.org_id,
        }
    }
}

#[derive(Serialize)]
pub struct AuthTokenDto {
    pub token: String,
}

#[derive(Serialize)]
pub struct AuthResponseDto {
    pub user: UserDto,
    pub token: String,
    pub org_id: i32,
    pub org_count: i32,
}

impl TryFrom<AuthResponseBuf> for AuthResponseDto {
    type Error = String;

    fn try_from(resp: AuthResponseBuf) -> std::result::Result<Self, Self::Error> {
        let Some(user) = resp.user else {
            return Err("Actor user should be present".to_string());
        };

        Ok(AuthResponseDto {
            user: user.into(),
            token: resp.token,
            org_id: resp.org_id,
            org_count: resp.org_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::datetime_now_str;

    use super::*;

    #[test]
    fn test_empty_actor() {
        let actor = Actor::default();
        assert!(!actor.has_auth_scope());
        assert!(!actor.is_system_admin());
    }

    #[test]
    fn test_regular_actor() {
        let today_str = datetime_now_str();
        let actor = Actor::new(
            ActorPayloadDto {
                id: 2000,
                org_id: 1000,
                org_count: 1,
                roles: vec![Role::OrgViewer],
                scopes: vec![Scope::Auth],
            },
            UserDto {
                id: 2001,
                email: "test".to_string(),
                name: "test".to_string(),
                status: "active".to_string(),
                created_at: today_str.clone(),
                updated_at: today_str.clone(),
            },
        );
        assert!(actor.has_auth_scope());
        assert!(!actor.is_system_admin());
    }

    #[test]
    fn test_system_admin_actor() {
        let today_str = datetime_now_str();
        let actor = Actor::new(
            ActorPayloadDto {
                id: 2000,
                org_id: 1000,
                org_count: 1,
                roles: vec![Role::Superuser],
                scopes: vec![Scope::Auth],
            },
            UserDto {
                id: 2001,
                email: "test".to_string(),
                name: "test".to_string(),
                status: "active".to_string(),
                created_at: today_str.clone(),
                updated_at: today_str.clone(),
            },
        );
        assert!(actor.has_auth_scope());
        assert!(actor.is_system_admin());
    }

    #[test]
    fn test_has_permissions_passes_when_actor_has_all_required() {
        let today_str = datetime_now_str();
        let actor = Actor::new(
            ActorPayloadDto {
                id: 2000,
                org_id: 1000,
                org_count: 1,
                roles: vec![Role::OrgViewer],
                scopes: vec![Scope::Auth],
            },
            UserDto {
                id: 2001,
                email: "test@example.com".to_string(),
                name: "test".to_string(),
                status: "active".to_string(),
                created_at: today_str.clone(),
                updated_at: today_str.clone(),
            },
        );

        let required = vec![Permission::OrgsView, Permission::UsersView];
        assert!(actor.has_permissions(&required));
    }

    #[test]
    fn test_has_permissions_fails_when_missing_required() {
        let today_str = datetime_now_str();
        let actor = Actor::new(
            ActorPayloadDto {
                id: 2000,
                org_id: 1000,
                org_count: 1,
                roles: vec![Role::OrgViewer],
                scopes: vec![Scope::Auth],
            },
            UserDto {
                id: 2001,
                email: "test@example.com".to_string(),
                name: "test".to_string(),
                status: "active".to_string(),
                created_at: today_str.clone(),
                updated_at: today_str.clone(),
            },
        );

        let required = vec![Permission::UsersDelete, Permission::UsersCreate];
        assert!(!actor.has_permissions(&required));
    }
}
