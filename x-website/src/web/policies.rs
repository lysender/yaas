use std::result::Result as StdResult;

use crate::{Error, Result};
use yaas::dto::Actor;
use yaas::role::Permission;

pub enum Resource {
    User,
    Org,
    App,
    OrgMember,
    OrgApp,
}

pub enum Action {
    Create,
    Read,
    Update,
    Delete,
}

pub fn enforce_policy(actor: &Actor, resource: Resource, action: Action) -> Result<()> {
    let result = match resource {
        Resource::User => enforce_users_permissions(actor, action),
        Resource::Org => enforce_orgs_permissions(actor, action),
        Resource::App => enforce_apps_permissions(actor, action),
        Resource::OrgMember => enforce_org_members_permissions(actor, action),
        Resource::OrgApp => enforce_org_apps_permissions(actor, action),
    };

    match result {
        Ok(_) => Ok(()),
        Err(message) => Err(Error::Forbidden {
            msg: message.to_string(),
        }),
    }
}

fn enforce_orgs_permissions(actor: &Actor, action: Action) -> StdResult<(), &str> {
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

fn enforce_apps_permissions(actor: &Actor, action: Action) -> StdResult<(), &str> {
    let (permissions, message) = match action {
        Action::Create => (
            vec![Permission::AppsCreate],
            "You do not have permission to create new apps.",
        ),
        Action::Read => (
            vec![Permission::AppsList, Permission::AppsView],
            "You do not have permission to view apps.",
        ),
        Action::Update => (
            vec![Permission::AppsEdit],
            "You do not have permission to edit apps.",
        ),
        Action::Delete => (
            vec![Permission::AppsDelete],
            "You do not have permission to delete apps.",
        ),
    };

    if !actor.has_permissions(&permissions) {
        return Err(message);
    }
    Ok(())
}

fn enforce_org_members_permissions(actor: &Actor, action: Action) -> StdResult<(), &str> {
    let (permissions, message) = match action {
        Action::Create => (
            vec![Permission::OrgMembersCreate],
            "You do not have permission to create new org members.",
        ),
        Action::Read => (
            vec![Permission::OrgMembersList, Permission::OrgMembersView],
            "You do not have permission to view org members.",
        ),
        Action::Update => (
            vec![Permission::OrgMembersEdit],
            "You do not have permission to edit org members.",
        ),
        Action::Delete => (
            vec![Permission::OrgMembersDelete],
            "You do not have permission to delete org members.",
        ),
    };

    if !actor.has_permissions(&permissions) {
        return Err(message);
    }
    Ok(())
}

fn enforce_org_apps_permissions(actor: &Actor, action: Action) -> StdResult<(), &str> {
    let (permissions, message) = match action {
        Action::Create => (
            vec![Permission::OrgAppsCreate],
            "You do not have permission to create new org apps.",
        ),
        Action::Read => (
            vec![Permission::OrgAppsList, Permission::OrgAppsView],
            "You do not have permission to view org apps.",
        ),
        Action::Update => (
            vec![Permission::OrgAppsEdit],
            "You do not have permission to edit org apps.",
        ),
        Action::Delete => (
            vec![Permission::OrgAppsDelete],
            "You do not have permission to delete org apps.",
        ),
    };

    if !actor.has_permissions(&permissions) {
        return Err(message);
    }
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
