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

    OrgsCreate,
    OrgsEdit,
    OrgsDelete,
    OrgsList,
    OrgsView,
    OrgsManage,

    UsersCreate,
    UsersEdit,
    UsersDelete,
    UsersList,
    UsersView,
    UsersManage,

    BucketsCreate,
    BucketsEdit,
    BucketsDelete,
    BucketsList,
    BucketsView,
    BucketsManage,

    DirsCreate,
    DirsEdit,
    DirsDelete,
    DirsList,
    DirsView,
    DirsManage,

    FilesCreate,
    FilesEdit,
    FilesDelete,
    FilesList,
    FilesView,
    FilesManage,
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
            "orgs.create" => Ok(Permission::OrgsCreate),
            "orgs.edit" => Ok(Permission::OrgsEdit),
            "orgs.delete" => Ok(Permission::OrgsDelete),
            "orgs.list" => Ok(Permission::OrgsList),
            "orgs.view" => Ok(Permission::OrgsView),
            "orgs.manage" => Ok(Permission::OrgsManage),
            "users.create" => Ok(Permission::UsersCreate),
            "users.edit" => Ok(Permission::UsersEdit),
            "users.delete" => Ok(Permission::UsersDelete),
            "users.list" => Ok(Permission::UsersList),
            "users.view" => Ok(Permission::UsersView),
            "users.manage" => Ok(Permission::UsersManage),
            "buckets.create" => Ok(Permission::BucketsCreate),
            "buckets.edit" => Ok(Permission::BucketsEdit),
            "buckets.delete" => Ok(Permission::BucketsDelete),
            "buckets.list" => Ok(Permission::BucketsList),
            "buckets.view" => Ok(Permission::BucketsView),
            "buckets.manage" => Ok(Permission::BucketsManage),
            "dirs.create" => Ok(Permission::DirsCreate),
            "dirs.edit" => Ok(Permission::DirsEdit),
            "dirs.delete" => Ok(Permission::DirsDelete),
            "dirs.list" => Ok(Permission::DirsList),
            "dirs.view" => Ok(Permission::DirsView),
            "dirs.manage" => Ok(Permission::DirsManage),
            "files.create" => Ok(Permission::FilesCreate),
            "files.edit" => Ok(Permission::FilesEdit),
            "files.delete" => Ok(Permission::FilesDelete),
            "files.list" => Ok(Permission::FilesList),
            "files.view" => Ok(Permission::FilesView),
            "files.manage" => Ok(Permission::FilesManage),
            _ => Err(format!("Invalid permission: {value}")),
        }
    }
}

impl TryFrom<i32> for Permission {
    type Error = String;

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Permission::Noop),
            10 => Ok(Permission::OrgsCreate),
            11 => Ok(Permission::OrgsEdit),
            12 => Ok(Permission::OrgsDelete),
            13 => Ok(Permission::OrgsList),
            14 => Ok(Permission::OrgsView),
            15 => Ok(Permission::OrgsManage),
            20 => Ok(Permission::UsersCreate),
            21 => Ok(Permission::UsersEdit),
            22 => Ok(Permission::UsersDelete),
            23 => Ok(Permission::UsersList),
            24 => Ok(Permission::UsersView),
            25 => Ok(Permission::UsersManage),
            30 => Ok(Permission::BucketsCreate),
            31 => Ok(Permission::BucketsEdit),
            32 => Ok(Permission::BucketsDelete),
            33 => Ok(Permission::BucketsList),
            34 => Ok(Permission::BucketsView),
            35 => Ok(Permission::BucketsManage),
            40 => Ok(Permission::DirsCreate),
            41 => Ok(Permission::DirsEdit),
            42 => Ok(Permission::DirsDelete),
            43 => Ok(Permission::DirsList),
            44 => Ok(Permission::DirsView),
            45 => Ok(Permission::DirsManage),
            50 => Ok(Permission::FilesCreate),
            51 => Ok(Permission::FilesEdit),
            52 => Ok(Permission::FilesDelete),
            53 => Ok(Permission::FilesList),
            54 => Ok(Permission::FilesView),
            55 => Ok(Permission::FilesManage),
            _ => Err(format!("Invalid permission: {value}")),
        }
    }
}

impl core::fmt::Display for Permission {
    fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
        match self {
            Permission::Noop => write!(f, "noop"),
            Permission::OrgsCreate => write!(f, "orgs.create"),
            Permission::OrgsEdit => write!(f, "orgs.edit"),
            Permission::OrgsDelete => write!(f, "orgs.delete"),
            Permission::OrgsList => write!(f, "orgs.list"),
            Permission::OrgsView => write!(f, "orgs.view"),
            Permission::OrgsManage => write!(f, "orgs.manage"),
            Permission::UsersCreate => write!(f, "users.create"),
            Permission::UsersEdit => write!(f, "users.edit"),
            Permission::UsersDelete => write!(f, "users.delete"),
            Permission::UsersList => write!(f, "users.list"),
            Permission::UsersView => write!(f, "users.view"),
            Permission::UsersManage => write!(f, "users.manage"),
            Permission::BucketsCreate => write!(f, "buckets.create"),
            Permission::BucketsEdit => write!(f, "buckets.edit"),
            Permission::BucketsDelete => write!(f, "buckets.delete"),
            Permission::BucketsList => write!(f, "buckets.list"),
            Permission::BucketsView => write!(f, "buckets.view"),
            Permission::BucketsManage => write!(f, "buckets.manage"),
            Permission::DirsCreate => write!(f, "dirs.create"),
            Permission::DirsEdit => write!(f, "dirs.edit"),
            Permission::DirsDelete => write!(f, "dirs.delete"),
            Permission::DirsList => write!(f, "dirs.list"),
            Permission::DirsView => write!(f, "dirs.view"),
            Permission::DirsManage => write!(f, "dirs.manage"),
            Permission::FilesCreate => write!(f, "files.create"),
            Permission::FilesEdit => write!(f, "files.edit"),
            Permission::FilesDelete => write!(f, "files.delete"),
            Permission::FilesList => write!(f, "files.list"),
            Permission::FilesView => write!(f, "files.view"),
            Permission::FilesManage => write!(f, "files.manage"),
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
            Permission::OrgsCreate => 10,
            Permission::OrgsEdit => 11,
            Permission::OrgsDelete => 12,
            Permission::OrgsList => 13,
            Permission::OrgsView => 14,
            Permission::OrgsManage => 15,
            Permission::UsersCreate => 20,
            Permission::UsersEdit => 21,
            Permission::UsersDelete => 22,
            Permission::UsersList => 23,
            Permission::UsersView => 24,
            Permission::UsersManage => 25,
            Permission::BucketsCreate => 30,
            Permission::BucketsEdit => 31,
            Permission::BucketsDelete => 32,
            Permission::BucketsList => 33,
            Permission::BucketsView => 34,
            Permission::BucketsManage => 35,
            Permission::DirsCreate => 40,
            Permission::DirsEdit => 41,
            Permission::DirsDelete => 42,
            Permission::DirsList => 43,
            Permission::DirsView => 44,
            Permission::DirsManage => 45,
            Permission::FilesCreate => 50,
            Permission::FilesEdit => 51,
            Permission::FilesDelete => 52,
            Permission::FilesList => 53,
            Permission::FilesView => 54,
            Permission::FilesManage => 55,
        })
        .collect()
}

/// Role to permissions mapping
pub fn role_permissions(role: &Role) -> Vec<Permission> {
    match role {
        Role::Superuser => vec![
            Permission::OrgsCreate,
            Permission::OrgsEdit,
            Permission::OrgsDelete,
            Permission::OrgsList,
            Permission::OrgsView,
            Permission::OrgsManage,
            Permission::UsersCreate,
            Permission::UsersEdit,
            Permission::UsersDelete,
            Permission::UsersList,
            Permission::UsersView,
            Permission::UsersManage,
            Permission::BucketsCreate,
            Permission::BucketsEdit,
            Permission::BucketsDelete,
            Permission::BucketsList,
            Permission::BucketsView,
            Permission::BucketsManage,
            Permission::DirsList,
            Permission::DirsView,
            Permission::FilesList,
            Permission::FilesView,
        ],
        Role::OrgAdmin => vec![
            Permission::OrgsList,
            Permission::OrgsView,
            Permission::BucketsList,
            Permission::BucketsView,
            Permission::UsersCreate,
            Permission::UsersEdit,
            Permission::UsersDelete,
            Permission::UsersList,
            Permission::UsersView,
            Permission::DirsCreate,
            Permission::DirsEdit,
            Permission::DirsDelete,
            Permission::DirsList,
            Permission::DirsView,
            Permission::DirsManage,
            Permission::FilesCreate,
            Permission::FilesEdit,
            Permission::FilesDelete,
            Permission::FilesList,
            Permission::FilesView,
            Permission::FilesManage,
        ],
        Role::OrgEditor => vec![
            Permission::OrgsList,
            Permission::OrgsView,
            Permission::BucketsList,
            Permission::BucketsView,
            Permission::DirsList,
            Permission::DirsView,
            Permission::FilesCreate,
            Permission::FilesList,
            Permission::FilesView,
        ],
        Role::OrgViewer => vec![
            Permission::OrgsList,
            Permission::OrgsView,
            Permission::BucketsList,
            Permission::BucketsView,
            Permission::DirsList,
            Permission::DirsView,
            Permission::FilesList,
            Permission::FilesView,
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
