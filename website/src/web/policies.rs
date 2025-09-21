use std::result::Result as StdResult;

use crate::{Error, Result};
use yaas::actor::Actor;
use yaas::role::Permission;

pub enum Resource {
    Client,
    Bucket,
    User,
    Album,
    Photo,
}

pub enum Action {
    Create,
    Read,
    Update,
    Delete,
}

pub fn enforce_policy(actor: &Actor, resource: Resource, action: Action) -> Result<()> {
    let result = match resource {
        Resource::Client => enforce_org_permissions(actor, action),
        Resource::Bucket => enforce_buckets_permissions(actor, action),
        Resource::User => enforce_users_permissions(actor, action),
        Resource::Album => enforce_dir_permissions(actor, action),
        Resource::Photo => enforce_photo_permissions(actor, action),
    };

    match result {
        Ok(_) => Ok(()),
        Err(message) => Err(Error::Forbidden {
            msg: message.to_string(),
        }),
    }
}

fn enforce_dir_permissions(actor: &Actor, action: Action) -> StdResult<(), &str> {
    Ok(())
}

fn enforce_photo_permissions(actor: &Actor, action: Action) -> StdResult<(), &str> {
    Ok(())
}

fn enforce_org_permissions(actor: &Actor, action: Action) -> StdResult<(), &str> {
    let (permissions, message) = match action {
        Action::Create => (
            vec![Permission::OrgsCreate],
            "You do not have permission to create new orgs.",
        ),
        Action::Read => (
            vec![Permission::OrgsList, Permission::OrgsView],
            "You do not have permission to view orgs.",
        ),
        Action::Update => (
            vec![Permission::OrgsEdit],
            "You do not have permission to edit orgs.",
        ),
        Action::Delete => (
            vec![Permission::OrgsDelete],
            "You do not have permission to delete orgs.",
        ),
    };

    if !actor.has_permissions(&permissions) {
        return Err(message);
    }
    Ok(())
}

fn enforce_buckets_permissions(actor: &Actor, action: Action) -> StdResult<(), &str> {
    Ok(())
}

fn enforce_users_permissions(actor: &Actor, action: Action) -> StdResult<(), &str> {
    let (permissions, message) = match action {
        Action::Create => (
            vec![Permission::UsersCreate],
            "You do not have permission to create new users.",
        ),
        Action::Read => (
            vec![Permission::UsersList, Permission::UsersView],
            "You do not have permission to view users.",
        ),
        Action::Update => (
            vec![Permission::UsersEdit],
            "You do not have permission to edit users.",
        ),
        Action::Delete => (
            vec![Permission::UsersDelete],
            "You do not have permission to delete users.",
        ),
    };

    if !actor.has_permissions(&permissions) {
        return Err(message);
    }
    Ok(())
}
