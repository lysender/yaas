# yaas

Yet another auth service

Objectives:
- Allow single sign on to multiple registered applications
- Organization is the key feature
- Manage users within the organization
- Email and password login by default
- Allows OAuth2 login with Google, Facebook, GitHub, etc

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

- Rust Backend
- REST

## Models

User:
- id
- email
- name
- status
- created_at
- updated_at
- deleted_at

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
- deleted_at

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
- deleted_at

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

## Yaas Frontend

## For System Admin

- [x] User management
- [x] App management
- [x] Org management
- [x] Org member management
- [x] Org app management

## For Org Admins/Users

- [x] Own org management
- [x] Own org member management
- [x] Own org app management

## OAuth for apps

- [x] GET `/oauth/authorize`
    - Query parameters: { client_id, redirect_uri, scope, state }
    - If not logged in, redirect to login page first then back to this endpoint
    - If there are validation errors, redirect to `redirect_uri` with error parameters: { error, error_description, state }
    - On success, redirect to `redirect_uri` with parameters: { code, state }
- [x] POST `/oauth/token`
    - Post payload: { client_id, client_secret, code, state, redirect_uri }
    - Response: { access_token, scope, token_type }

## Yaas API

Setup Endpoints:
- [x] GET `/setup`
- [x] POST `/setup`

Auth Endpoints (for users):
- [x] POST `/auth/authorize`

OAuth Endpoints (for apps):
- [x] POST `/oauth/authorize`
    - User must be authorized first
    - Post payload: { client_id, redirect_uri, scope, state }
    - Response: { code, state }
- [x] POST `/oauth/token`
    - Post payload: { client_id, client_secret, code, redirect_uri }
    - Response: { access_token, scope, token_type }

Health Endpoints:
- [x] GET `/health/live`
    - Response: `{ "status": "UP" }`
    - Returns `200` when process is alive
- [x] GET `/health/ready`
    - Response: `{ "status": "UP|DOWN", "message": "...", "checks": { "database": "UP|DOWN" } }`
    - Returns `200` when all readiness checks pass, otherwise `503`

Kubernetes probe example:

```yaml
livenessProbe:
  httpGet:
    path: /health/live
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 10

readinessProbe:
  httpGet:
    path: /health/ready
    port: 8080
  initialDelaySeconds: 5
  periodSeconds: 10
```

User Endpoints:
- [x] GET `/user`
- [x] GET `/user/authz`
- [x] POST `/user/change-password`
- [x] PUT `/user/auth-context`

Users Endpoints:
- [x] GET `/users`
- [x] POST `/users`
- [x] GET `/users/{user_id}`
- [x] PATCH `/users/{user_id}`
- [x] PUT `/users/{user_id}/password`
- [x] DELETE `/users/{user_id}`

Apps Endpoints:
- [x] GET `/apps`
- [x] POST `/apps`
- [x] GET `/apps/{app_id}`
- [x] PATCH `/apps/{app_id}`
- [x] DELETE `/apps/{app_id}`

Orgs Endpoints:
- [x] GET `/orgs`
- [x] POST `/orgs`
- [x] GET `/orgs/{org_id}`
- [x] PATCH `/orgs/{org_id}`
- [x] DELETE `/orgs/{org_id}`

Org Members Endpoints:
- [x] GET `/orgs/{org_id}/members`
- [x] POST `/orgs/{org_id}/members`
- [x] GET `/orgs/{org_id}/members/{user_id}`
- [x] PATCH `/orgs/{org_id}/members/{user_id}`
- [x] DELETE `/orgs/{org_id}/members/{user_id}`
- [x] GET `/orgs/{org_id}/member-suggestions`

Org Apps Endpoints:
- [x] GET `/orgs/{org_id}/apps`
- [x] POST `/orgs/{org_id}/apps`
- [x] GET `/orgs/{org_id}/apps/{app_id}`
- [x] DELETE `/orgs/{org_id}/apps/{app_id}`
- [x] GET `/orgs/{org_id}/app-suggestions`

2026-03-31 Objectives:
- [ ] Merge API and Website app into one app
- [ ] Migrate smoke tests to bin runner
