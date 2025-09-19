use serde::{Deserialize, Serialize};
use snafu::{Snafu, ensure};
use std::collections::HashSet;

#[derive(PartialEq, Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    Superuser,
    OrgAdmin,
    OrgEditor,
    OrgViewer,
}

#[derive(Debug, Snafu)]
#[snafu(display("Invalid roles: {roles}"))]
pub struct InvalidRolesError {
    roles: String,
}

#[derive(PartialEq, Eq, Hash, Debug, Clone, Serialize, Deserialize)]
pub enum Permission {
    Noop,

    UsersCreate,
    UsersEdit,
    UsersDelete,
    UsersList,
    UsersView,
    UsersManage,

    AppsCreate,
    AppsEdit,
    AppsDelete,
    AppsList,
    AppsView,
    AppsManage,

    OrgsCreate,
    OrgsEdit,
    OrgsDelete,
    OrgsList,
    OrgsView,
    OrgsManage,

    OrgMembersCreate,
    OrgMembersEdit,
    OrgMembersDelete,
    OrgMembersList,
    OrgMembersView,
    OrgMembersManage,

    OrgAppsCreate,
    OrgAppsEdit,
    OrgAppsDelete,
    OrgAppsList,
    OrgAppsView,
    OrgAppsManage,
}

#[derive(Debug, Snafu)]
#[snafu(display("Invalid permissions: {permissions}"))]
pub struct InvalidPermissionsError {
    permissions: String,
}

impl TryFrom<&str> for Role {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Superuser" => Ok(Role::Superuser),
            "OrgAdmin" => Ok(Role::OrgAdmin),
            "OrgEditor" => Ok(Role::OrgEditor),
            "OrgViewer" => Ok(Role::OrgViewer),
            _ => Err(format!("Invalid role: {value}")),
        }
    }
}

impl TryFrom<i32> for Role {
    type Error = String;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Role::Superuser),
            10 => Ok(Role::OrgAdmin),
            20 => Ok(Role::OrgEditor),
            30 => Ok(Role::OrgViewer),
            _ => Err(format!("Invalid role: {value}")),
        }
    }
}

impl core::fmt::Display for Role {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Role::Superuser => write!(f, "Superuser"),
            Role::OrgAdmin => write!(f, "OrgAdmin"),
            Role::OrgEditor => write!(f, "OrgEditor"),
            Role::OrgViewer => write!(f, "OrgViewer"),
        }
    }
}

pub fn to_roles(list: &Vec<String>) -> Result<Vec<Role>, InvalidRolesError> {
    let mut roles: Vec<Role> = Vec::with_capacity(list.len());
    let mut errors: Vec<String> = Vec::with_capacity(list.len());
    for item in list.iter() {
        let role = item.as_str();
        match Role::try_from(role) {
            Ok(role) => roles.push(role),
            Err(_) => errors.push(role.to_string()),
        }
    }

    ensure!(
        errors.len() == 0,
        InvalidRolesSnafu {
            roles: errors.join(", ")
        }
    );

    Ok(roles)
}

pub fn to_buffed_roles(list: &Vec<Role>) -> Vec<i32> {
    list.iter()
        .map(|role| match role {
            Role::Superuser => 0,
            Role::OrgAdmin => 10,
            Role::OrgEditor => 20,
            Role::OrgViewer => 30,
        })
        .collect()
}

pub fn buffed_to_roles(list: &Vec<i32>) -> Result<Vec<Role>, InvalidRolesError> {
    let mut roles: Vec<Role> = Vec::with_capacity(list.len());
    let mut errors: Vec<String> = Vec::with_capacity(list.len());
    for item in list.iter() {
        match Role::try_from(*item) {
            Ok(role) => roles.push(role),
            Err(_) => errors.push(item.to_string()),
        }
    }

    ensure!(
        errors.len() == 0,
        InvalidRolesSnafu {
            roles: errors.join(", ")
        }
    );

    Ok(roles)
}

impl TryFrom<&str> for Permission {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "noop" => Ok(Permission::Noop),

            "users.create" => Ok(Permission::UsersCreate),
            "users.edit" => Ok(Permission::UsersEdit),
            "users.delete" => Ok(Permission::UsersDelete),
            "users.list" => Ok(Permission::UsersList),
            "users.view" => Ok(Permission::UsersView),
            "users.manage" => Ok(Permission::UsersManage),

            "apps.create" => Ok(Permission::AppsCreate),
            "apps.edit" => Ok(Permission::AppsEdit),
            "apps.delete" => Ok(Permission::AppsDelete),
            "apps.list" => Ok(Permission::AppsList),
            "apps.view" => Ok(Permission::AppsView),
            "apps.manage" => Ok(Permission::AppsManage),

            "orgs.create" => Ok(Permission::OrgsCreate),
            "orgs.edit" => Ok(Permission::OrgsEdit),
            "orgs.delete" => Ok(Permission::OrgsDelete),
            "orgs.list" => Ok(Permission::OrgsList),
            "orgs.view" => Ok(Permission::OrgsView),
            "orgs.manage" => Ok(Permission::OrgsManage),

            "org_members.create" => Ok(Permission::OrgMembersCreate),
            "org_members.edit" => Ok(Permission::OrgMembersEdit),
            "org_members.delete" => Ok(Permission::OrgMembersDelete),
            "org_members.list" => Ok(Permission::OrgMembersList),
            "org_members.view" => Ok(Permission::OrgMembersView),
            "org_members.manage" => Ok(Permission::OrgMembersManage),

            "org_apps.create" => Ok(Permission::OrgAppsCreate),
            "org_apps.edit" => Ok(Permission::OrgAppsEdit),
            "org_apps.delete" => Ok(Permission::OrgAppsDelete),
            "org_apps.list" => Ok(Permission::OrgAppsList),
            "org_apps.view" => Ok(Permission::OrgAppsView),
            "org_apps.manage" => Ok(Permission::OrgAppsManage),

            _ => Err(format!("Invalid permission: {value}")),
        }
    }
}

impl TryFrom<i32> for Permission {
    type Error = String;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Permission::Noop),

            10 => Ok(Permission::UsersCreate),
            11 => Ok(Permission::UsersEdit),
            12 => Ok(Permission::UsersDelete),
            13 => Ok(Permission::UsersList),
            14 => Ok(Permission::UsersView),
            15 => Ok(Permission::UsersManage),

            20 => Ok(Permission::AppsCreate),
            21 => Ok(Permission::AppsEdit),
            22 => Ok(Permission::AppsDelete),
            23 => Ok(Permission::AppsList),
            24 => Ok(Permission::AppsView),
            25 => Ok(Permission::AppsManage),

            30 => Ok(Permission::OrgsCreate),
            31 => Ok(Permission::OrgsEdit),
            32 => Ok(Permission::OrgsDelete),
            33 => Ok(Permission::OrgsList),
            34 => Ok(Permission::OrgsView),
            35 => Ok(Permission::OrgsManage),

            40 => Ok(Permission::OrgMembersCreate),
            41 => Ok(Permission::OrgMembersEdit),
            42 => Ok(Permission::OrgMembersDelete),
            43 => Ok(Permission::OrgMembersList),
            44 => Ok(Permission::OrgMembersView),
            45 => Ok(Permission::OrgMembersManage),

            50 => Ok(Permission::OrgAppsCreate),
            51 => Ok(Permission::OrgAppsEdit),
            52 => Ok(Permission::OrgAppsDelete),
            53 => Ok(Permission::OrgAppsList),
            54 => Ok(Permission::OrgAppsView),
            55 => Ok(Permission::OrgAppsManage),

            _ => Err(format!("Invalid permission: {value}")),
        }
    }
}

impl core::fmt::Display for Permission {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Permission::Noop => write!(f, "noop"),

            Permission::UsersCreate => write!(f, "users.create"),
            Permission::UsersEdit => write!(f, "users.edit"),
            Permission::UsersDelete => write!(f, "users.delete"),
            Permission::UsersList => write!(f, "users.list"),
            Permission::UsersView => write!(f, "users.view"),
            Permission::UsersManage => write!(f, "users.manage"),

            Permission::AppsCreate => write!(f, "apps.create"),
            Permission::AppsEdit => write!(f, "apps.edit"),
            Permission::AppsDelete => write!(f, "apps.delete"),
            Permission::AppsList => write!(f, "apps.list"),
            Permission::AppsView => write!(f, "apps.view"),
            Permission::AppsManage => write!(f, "apps.manage"),

            Permission::OrgsCreate => write!(f, "orgs.create"),
            Permission::OrgsEdit => write!(f, "orgs.edit"),
            Permission::OrgsDelete => write!(f, "orgs.delete"),
            Permission::OrgsList => write!(f, "orgs.list"),
            Permission::OrgsView => write!(f, "orgs.view"),
            Permission::OrgsManage => write!(f, "orgs.manage"),

            Permission::OrgMembersCreate => write!(f, "org_members.create"),
            Permission::OrgMembersEdit => write!(f, "org_members.edit"),
            Permission::OrgMembersDelete => write!(f, "org_members.delete"),
            Permission::OrgMembersList => write!(f, "org_members.list"),
            Permission::OrgMembersView => write!(f, "org_members.view"),
            Permission::OrgMembersManage => write!(f, "org_members.manage"),

            Permission::OrgAppsCreate => write!(f, "org_apps.create"),
            Permission::OrgAppsEdit => write!(f, "org_apps.edit"),
            Permission::OrgAppsDelete => write!(f, "org_apps.delete"),
            Permission::OrgAppsList => write!(f, "org_apps.list"),
            Permission::OrgAppsView => write!(f, "org_apps.view"),
            Permission::OrgAppsManage => write!(f, "org_apps.manage"),
        }
    }
}

pub fn to_permissions(
    permissions: &Vec<String>,
) -> Result<Vec<Permission>, InvalidPermissionsError> {
    let mut perms: Vec<Permission> = Vec::with_capacity(permissions.len());
    let mut errors: Vec<String> = Vec::with_capacity(permissions.len());
    for item in permissions.iter() {
        let perm = item.as_str();
        match Permission::try_from(perm) {
            Ok(permission) => perms.push(permission),
            Err(_) => errors.push(perm.to_string()),
        }
    }

    ensure!(
        errors.len() == 0,
        InvalidPermissionsSnafu {
            permissions: errors.join(", ")
        }
    );

    Ok(perms)
}

pub fn buffed_to_permissions(list: &Vec<i32>) -> Result<Vec<Permission>, InvalidPermissionsError> {
    let mut perms: Vec<Permission> = Vec::with_capacity(list.len());
    let mut errors: Vec<String> = Vec::with_capacity(list.len());
    for item in list.iter() {
        match Permission::try_from(*item) {
            Ok(permission) => perms.push(permission),
            Err(_) => errors.push(item.to_string()),
        }
    }

    ensure!(
        errors.len() == 0,
        InvalidPermissionsSnafu {
            permissions: errors.join(", ")
        }
    );

    Ok(perms)
}

pub fn to_buffed_permissions(list: &Vec<Permission>) -> Vec<i32> {
    list.iter()
        .map(|perm| match perm {
            Permission::Noop => 0,

            Permission::UsersCreate => 10,
            Permission::UsersEdit => 11,
            Permission::UsersDelete => 12,
            Permission::UsersList => 13,
            Permission::UsersView => 14,
            Permission::UsersManage => 15,

            Permission::AppsCreate => 20,
            Permission::AppsEdit => 21,
            Permission::AppsDelete => 22,
            Permission::AppsList => 23,
            Permission::AppsView => 24,
            Permission::AppsManage => 25,

            Permission::OrgsCreate => 30,
            Permission::OrgsEdit => 31,
            Permission::OrgsDelete => 32,
            Permission::OrgsList => 33,
            Permission::OrgsView => 34,
            Permission::OrgsManage => 35,

            Permission::OrgMembersCreate => 40,
            Permission::OrgMembersEdit => 41,
            Permission::OrgMembersDelete => 42,
            Permission::OrgMembersList => 43,
            Permission::OrgMembersView => 44,
            Permission::OrgMembersManage => 45,

            Permission::OrgAppsCreate => 50,
            Permission::OrgAppsEdit => 51,
            Permission::OrgAppsDelete => 52,
            Permission::OrgAppsList => 53,
            Permission::OrgAppsView => 54,
            Permission::OrgAppsManage => 55,
        })
        .collect()
}

/// Role to permissions mapping
pub fn role_permissions(role: &Role) -> Vec<Permission> {
    match role {
        Role::Superuser => vec![
            Permission::UsersCreate,
            Permission::UsersEdit,
            Permission::UsersDelete,
            Permission::UsersList,
            Permission::UsersView,
            Permission::UsersManage,
            Permission::AppsCreate,
            Permission::AppsEdit,
            Permission::AppsDelete,
            Permission::AppsList,
            Permission::AppsView,
            Permission::AppsManage,
            Permission::OrgsCreate,
            Permission::OrgsEdit,
            Permission::OrgsDelete,
            Permission::OrgsList,
            Permission::OrgsView,
            Permission::OrgsManage,
            Permission::OrgMembersCreate,
            Permission::OrgMembersEdit,
            Permission::OrgMembersDelete,
            Permission::OrgMembersList,
            Permission::OrgMembersView,
            Permission::OrgMembersManage,
            Permission::OrgAppsCreate,
            Permission::OrgAppsEdit,
            Permission::OrgAppsDelete,
            Permission::OrgAppsList,
            Permission::OrgAppsView,
            Permission::OrgAppsManage,
        ],
        Role::OrgAdmin => vec![
            Permission::UsersList,
            Permission::UsersView,
            Permission::OrgsEdit,
            Permission::OrgsDelete,
            Permission::OrgsList,
            Permission::OrgsView,
            Permission::OrgsManage,
            Permission::OrgMembersCreate,
            Permission::OrgMembersEdit,
            Permission::OrgMembersDelete,
            Permission::OrgMembersList,
            Permission::OrgMembersView,
            Permission::OrgMembersManage,
            Permission::OrgAppsList,
            Permission::OrgAppsView,
        ],
        Role::OrgEditor => vec![
            Permission::UsersList,
            Permission::UsersView,
            Permission::OrgsList,
            Permission::OrgsView,
            Permission::OrgMembersCreate,
            Permission::OrgMembersEdit,
            Permission::OrgMembersDelete,
            Permission::OrgMembersList,
            Permission::OrgMembersView,
            Permission::OrgAppsList,
            Permission::OrgAppsView,
        ],
        Role::OrgViewer => vec![
            Permission::UsersView,
            Permission::OrgsList,
            Permission::OrgsView,
            Permission::OrgMembersList,
            Permission::OrgMembersView,
            Permission::OrgAppsList,
            Permission::OrgAppsView,
        ],
    }
}

/// Get all permissions for the given roles
pub fn roles_permissions(roles: &Vec<Role>) -> Vec<Permission> {
    let mut permissions: HashSet<Permission> = HashSet::new();
    roles.iter().for_each(|role| {
        role_permissions(role).iter().for_each(|p| {
            permissions.insert(p.clone());
        });
    });
    permissions.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_roles_valid() {
        let data = vec!["OrgAdmin".to_string(), "OrgViewer".to_string()];
        let roles = to_roles(&data).unwrap();
        assert_eq!(roles, vec![Role::OrgAdmin, Role::OrgViewer]);
    }

    #[test]
    fn test_to_roles_invalid() {
        let data = vec![
            "OrgAdmin".to_string(),
            "InvalidRole".to_string(),
            "NetflixRole".to_string(),
        ];
        let roles = to_roles(&data);
        assert!(roles.is_err());
        if let Err(e) = roles {
            assert_eq!(e.to_string(), "Invalid roles: InvalidRole, NetflixRole");
        }
    }

    #[test]
    fn test_to_permissions_valid() {
        let data = vec![
            "orgs.create".to_string(),
            "orgs.edit".to_string(),
            "orgs.delete".to_string(),
        ];
        let permissions = to_permissions(&data).unwrap();
        assert_eq!(
            permissions,
            vec![
                Permission::OrgsCreate,
                Permission::OrgsEdit,
                Permission::OrgsDelete,
            ]
        );
    }

    #[test]
    fn test_to_permissions_invalid() {
        let data = vec![
            "orgs.create".to_string(),
            "orgs.edit".to_string(),
            "orgs.delete".to_string(),
            "netflix.binge".to_string(),
            "netflix.watch".to_string(),
        ];
        let permissions = to_permissions(&data);
        assert!(permissions.is_err());
        if let Err(e) = permissions {
            assert_eq!(
                e.to_string(),
                "Invalid permissions: netflix.binge, netflix.watch"
            );
        }
    }
}
