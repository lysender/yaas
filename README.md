# yaas

Yet another auth service

Objectives:
- Allow single sign on to multiple registered applications
- Organization is the key feature
- Manage users within the organization
- Email and password login by default
- Allows OAuth2 login with Google, Facebook, GitHub, etc.

## Workflow

- User visits an application
- Application redirects to yaas
- yaas let the user login
- yaas redirects back to the application with an authorization code
- application exchanges the authorization code for an access token
- application uses the access token to access the user's information
- application store access token in cookie
- application let's the user access the application

## Super Admin Setup

- There must be a process where a super admin is created
- The application should not be accessible until the super admin is created

## Tech Stack

- Rust
- Protobuf
- REST

## Models

User:
- id
- email
- name
- status
- created_at
- updated_at

Password:
- id
- password
- created_at
- updated_at

Org:
- id
- name
- status
- owner_id
- created_at
- updated_at

OrgMember:
- id
- org_id
- user_id
- roles
- status
- created_at
- updated_at

App:
- id
- name
- secret
- redirect_uri
- created_at
- updated_at

OrgApp:
- id
- org_id
- app_id
- created_at

OauthCode:
- id
- code
- state
- redirect_uri
- scope
- app_id
- org_id
- user_id
- created_at
- expires_at

## Roles

- SuperAdmin
- OrgAdmin
- OrgMember
