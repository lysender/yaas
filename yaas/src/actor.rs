use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::dto::UserDto;
use crate::role::{Permission, Role, roles_permissions, to_permissions};

#[derive(Clone)]
pub struct ActorPayload {
    pub id: String,
    pub org_id: String,
    pub roles: Vec<Role>,
    pub scope: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Actor {
    pub actor: Option<ActorDto>,
    pub scope: String,
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
            Some(actor) => actor.scope.contains(scope),
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
    #[validate(length(min = 1, max = 100))]
    #[validate(email)]
    pub email: String,

    #[validate(length(min = 8, max = 60))]
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthToken {
    pub token: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub user: UserDto,
    pub token: String,
}

#[cfg(test)]
mod tests {
    use crate::utils::{datetime_now_str, generate_id};

    use super::*;

    #[test]
    fn test_empty_actor() {
        let actor = Actor::default();
        assert_eq!(actor.has_auth_scope(), false);
        assert_eq!(actor.is_system_admin(), false);
    }

    #[test]
    fn test_regular_actor() {
        let org_id = generate_id("org");
        let today_str = datetime_now_str();
        let actor = Actor::new(
            ActorPayload {
                id: generate_id("usr"),
                org_id: org_id.clone(),
                roles: vec![Role::OrgEditor],
                scope: "auth".to_string(),
            },
            UserDto {
                id: generate_id("usr"),
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
        let org_id = generate_id("org");
        let today_str = datetime_now_str();
        let actor = Actor::new(
            ActorPayload {
                id: generate_id("usr"),
                org_id: org_id.clone(),
                roles: vec![Role::Superuser],
                scope: "auth".to_string(),
            },
            UserDto {
                id: generate_id("usr"),
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
