use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::buffed::actor::{ActorBuf, AuthResponseBuf, CredentialsBuf};
use crate::role::{
    Permission, Role, buffed_to_permissions, buffed_to_roles, roles_permissions, to_permissions,
};
use crate::xdto::{OrgMembershipDto, UserDto};

#[derive(Clone, Serialize, Deserialize)]
pub struct ActorDto {
    pub id: i32,
    pub org_id: i32,
    pub scope: String,
    pub user: UserDto,
    pub roles: Vec<Role>,
    pub permissions: Vec<Permission>,
}

impl TryFrom<ActorBuf> for ActorDto {
    type Error = String;

    fn try_from(actor: ActorBuf) -> Result<Self, Self::Error> {
        let Ok(roles) = buffed_to_roles(&actor.roles) else {
            return Err("Actor roles should convert back to enum".to_string());
        };
        let Ok(permissions) = buffed_to_permissions(&actor.permissions) else {
            return Err("Actor permissions should convert back to enum".to_string());
        };

        let Some(user) = actor.user else {
            return Err("Actor user should be present".to_string());
        };

        Ok(ActorDto {
            id: actor.id,
            org_id: actor.org_id,
            scope: actor.scope,
            user: user.into(),
            roles,
            permissions,
        })
    }
}

#[derive(Clone)]
pub struct ActorPayload {
    pub id: i32,
    pub org_id: i32,
    pub roles: Vec<Role>,
    pub scope: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Actor {
    pub actor: Option<ActorDto>,
}

impl Actor {
    pub fn new(payload: ActorPayload, user: UserDto) -> Self {
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
                scope: payload.scope,
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
        self.has_scope("auth")
    }

    pub fn has_vault_scope(&self) -> bool {
        self.has_scope("vault")
    }

    pub fn has_scope(&self, scope: &str) -> bool {
        match &self.actor {
            Some(actor) => {
                let scopes: Vec<&str> = actor.scope.split(' ').collect();
                scopes.contains(&scope)
            }
            None => false,
        }
    }

    pub fn has_permissions(&self, permissions: &Vec<Permission>) -> bool {
        match &self.actor {
            Some(actor) => actor
                .permissions
                .iter()
                .any(|permission| permissions.contains(permission)),
            None => false,
        }
    }

    pub fn is_system_admin(&self) -> bool {
        match &self.actor {
            Some(actor) => actor.roles.iter().any(|role| *role == Role::Superuser),
            None => false,
        }
    }
}

#[derive(Deserialize, Serialize, Validate)]
pub struct Credentials {
    #[validate(length(max = 100))]
    #[validate(email)]
    pub email: String,

    #[validate(length(min = 8, max = 60))]
    pub password: String,
}

impl From<CredentialsBuf> for Credentials {
    fn from(creds: CredentialsBuf) -> Self {
        Credentials {
            email: creds.email,
            password: creds.password,
        }
    }
}

#[derive(Serialize)]
pub struct AuthToken {
    pub token: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub user: UserDto,

    /// Present auth is good to go
    pub token: Option<String>,

    /// Present when user has to select one of their orgs
    /// Token can only be used as part of the auth flow
    pub select_org_token: Option<String>,
    pub select_org_options: Vec<OrgMembershipDto>,
}

impl TryFrom<AuthResponseBuf> for AuthResponse {
    type Error = String;

    fn try_from(resp: AuthResponseBuf) -> Result<Self, Self::Error> {
        let Some(user) = resp.user else {
            return Err("Actor user should be present".to_string());
        };

        let org_len = resp.select_org_options.len();
        let orgs: Vec<OrgMembershipDto> = resp
            .select_org_options
            .into_iter()
            .map(|m| {
                let m: Result<OrgMembershipDto, String> = m.try_into();
                m.ok()
            })
            .flat_map(|x| x)
            .collect();

        if orgs.len() != org_len {
            return Err("Org membership should convert back to enum".to_string());
        }

        Ok(AuthResponse {
            user: user.into(),
            token: resp.token,
            select_org_token: resp.select_org_token,
            select_org_options: orgs,
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
        assert_eq!(actor.has_auth_scope(), false);
        assert_eq!(actor.is_system_admin(), false);
    }

    #[test]
    fn test_regular_actor() {
        let org_id = 1000;
        let today_str = datetime_now_str();
        let actor = Actor::new(
            ActorPayload {
                id: 2000,
                org_id: org_id,
                roles: vec![Role::OrgEditor],
                scope: "auth".to_string(),
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
        assert_eq!(actor.has_auth_scope(), true);
        assert_eq!(actor.is_system_admin(), false);
    }

    #[test]
    fn test_system_admin_actor() {
        let org_id = 1000;
        let today_str = datetime_now_str();
        let actor = Actor::new(
            ActorPayload {
                id: 2000,
                org_id: org_id,
                roles: vec![Role::Superuser],
                scope: "auth".to_string(),
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
        assert_eq!(actor.has_auth_scope(), true);
        assert_eq!(actor.is_system_admin(), true);
    }
}
