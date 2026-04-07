use serde::{Deserialize, Serialize};
use validator::Validate;

use crate::dto::UserDto;
use crate::dto::{Permission, Role, Scope, roles_permissions, to_permissions};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ActorDto {
    pub id: String,
    pub org_id: String,
    pub org_count: i32,
    pub scopes: Vec<Scope>,
    pub user: UserDto,
    pub roles: Vec<Role>,
    pub permissions: Vec<Permission>,
}

#[derive(Clone)]
pub struct ActorPayloadDto {
    pub id: String,
    pub org_id: String,
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

    pub fn member_of(&self, org_id: &str) -> bool {
        match &self.actor {
            Some(actor) => actor.org_id == org_id,
            None => false,
        }
    }
}

impl Default for Actor {
    /// Empty actor for unauthenticated requests
    fn default() -> Self {
        Actor { actor: None }
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

#[derive(Deserialize, Serialize, Validate)]
pub struct SwitchAuthContextDto {
    pub org_id: String,
}

#[derive(Serialize, Deserialize)]
pub struct AuthResponseDto {
    pub user: UserDto,
    pub token: String,
    pub org_id: String,
    pub org_count: i32,
}

#[cfg(test)]
mod tests {
    use crate::utils::{IdPrefix, datetime_now_millis, generate_id};

    use super::*;

    #[test]
    fn test_empty_actor() {
        let actor = Actor::default();
        assert!(!actor.has_auth_scope());
        assert!(!actor.is_system_admin());
    }

    #[test]
    fn test_regular_actor() {
        let today = datetime_now_millis();
        let user_id = generate_id(IdPrefix::User);
        let actor = Actor::new(
            ActorPayloadDto {
                id: user_id.clone(),
                org_id: generate_id(IdPrefix::Org),
                org_count: 1,
                roles: vec![Role::OrgViewer],
                scopes: vec![Scope::Auth],
            },
            UserDto {
                id: user_id,
                email: "test".to_string(),
                name: "test".to_string(),
                status: "active".to_string(),
                created_at: today,
                updated_at: today,
            },
        );
        assert!(actor.has_auth_scope());
        assert!(!actor.is_system_admin());
    }

    #[test]
    fn test_system_admin_actor() {
        let today = datetime_now_millis();
        let user_id = generate_id(IdPrefix::User);
        let actor = Actor::new(
            ActorPayloadDto {
                id: user_id.clone(),
                org_id: generate_id(IdPrefix::Org),
                org_count: 1,
                roles: vec![Role::Superuser],
                scopes: vec![Scope::Auth],
            },
            UserDto {
                id: user_id,
                email: "test".to_string(),
                name: "test".to_string(),
                status: "active".to_string(),
                created_at: today,
                updated_at: today,
            },
        );
        assert!(actor.has_auth_scope());
        assert!(actor.is_system_admin());
    }

    #[test]
    fn test_has_permissions_passes_when_actor_has_all_required() {
        let today = datetime_now_millis();
        let user_id = generate_id(IdPrefix::User);
        let actor = Actor::new(
            ActorPayloadDto {
                id: user_id.clone(),
                org_id: generate_id(IdPrefix::Org),
                org_count: 1,
                roles: vec![Role::OrgViewer],
                scopes: vec![Scope::Auth],
            },
            UserDto {
                id: user_id,
                email: "test@example.com".to_string(),
                name: "test".to_string(),
                status: "active".to_string(),
                created_at: today,
                updated_at: today,
            },
        );

        let required = vec![Permission::OrgsView, Permission::UsersView];
        assert!(actor.has_permissions(&required));
    }

    #[test]
    fn test_has_permissions_fails_when_missing_required() {
        let today = datetime_now_millis();
        let user_id = generate_id(IdPrefix::User);
        let actor = Actor::new(
            ActorPayloadDto {
                id: user_id.clone(),
                org_id: generate_id(IdPrefix::Org),
                org_count: 1,
                roles: vec![Role::OrgViewer],
                scopes: vec![Scope::Auth],
            },
            UserDto {
                id: user_id,
                email: "test@example.com".to_string(),
                name: "test".to_string(),
                status: "active".to_string(),
                created_at: today,
                updated_at: today,
            },
        );

        let required = vec![Permission::UsersDelete, Permission::UsersCreate];
        assert!(!actor.has_permissions(&required));
    }
}
