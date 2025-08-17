use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::dto::UserDto;
use crate::role::{Permission, Role, roles_permissions, to_permissions};

#[derive(Clone)]
pub struct ActorPayload {
    pub id: String,
    pub org_id: String,
    pub scope: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Actor {
    pub id: String,
    pub org_id: String,
    pub scope: String,
    pub user: UserDto,
    pub roles: Vec<Role>,
    pub permissions: Vec<Permission>,
}

impl Actor {
    pub fn new(payload: ActorPayload, user: UserDto) -> Self {
        let roles = Vec::new();
        let permissions: Vec<Permission> = roles_permissions(&roles).into_iter().collect();
        // Convert to string to allow sorting
        let mut permissions: Vec<String> = permissions.iter().map(|p| p.to_string()).collect();
        permissions.sort();
        // Convert again to Permission enum
        let permissions: Vec<Permission> =
            to_permissions(&permissions).expect("Invalid permissions");

        Actor {
            id: user.id.clone(),
            org_id: payload.org_id,
            scope: payload.scope,
            user,
            roles,
            permissions,
        }
    }

    /// Empty actor for unauthenticated requests
    pub fn empty() -> Self {
        Actor {
            id: "unknown".to_string(),
            org_id: "unknown".to_string(),
            scope: "".to_string(),
            user: UserDto {
                id: "unknown".to_string(),
                email: "unknown".to_string(),
                name: "unknown".to_string(),
                status: "unknown".to_string(),
                created_at: "".to_string(),
                updated_at: "".to_string(),
                deleted_at: None,
            },
            roles: vec![],
            permissions: vec![],
        }
    }

    pub fn has_auth_scope(&self) -> bool {
        self.has_scope("auth")
    }

    pub fn has_vault_scope(&self) -> bool {
        self.has_scope("vault")
    }

    pub fn has_scope(&self, scope: &str) -> bool {
        self.scope.contains(scope)
    }

    pub fn has_permissions(&self, permissions: &Vec<Permission>) -> bool {
        permissions
            .iter()
            .all(|permission| self.permissions.contains(permission))
    }

    pub fn is_system_admin(&self) -> bool {
        self.roles
            .iter()
            .find(|role| **role == Role::SystemAdmin)
            .is_some()
    }
}

#[derive(Deserialize, Serialize, Validate)]
pub struct Credentials {
    #[validate(length(min = 1, max = 30))]
    pub username: String,

    #[validate(length(min = 8, max = 100))]
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
        let actor = Actor::empty();
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
                scope: "auth".to_string(),
            },
            UserDto {
                id: generate_id("usr"),
                email: "test".to_string(),
                name: "test".to_string(),
                status: "active".to_string(),
                created_at: today_str.clone(),
                updated_at: today_str.clone(),
                deleted_at: None,
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
                scope: "auth".to_string(),
            },
            UserDto {
                id: generate_id("usr"),
                email: "test".to_string(),
                name: "test".to_string(),
                status: "active".to_string(),
                created_at: today_str.clone(),
                updated_at: today_str.clone(),
                deleted_at: None,
            },
        );
        assert_eq!(actor.has_auth_scope(), true);
        assert_eq!(actor.is_system_admin(), true);
    }
}
