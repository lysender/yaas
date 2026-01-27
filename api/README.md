# Yaas API

Setup Endpoints:
- [x] POST `/setup`

Auth Endpoints (for users):
- [x] POST `/auth/authorize`

OAuth Endpoints (for apps):
- [x] POST `/oauth/authorize`
    - User must be authorized first
    - Post payload: { client_id, redirect_uri, scope, state }
    - Response: { code, state }
- [ ] POST `/oauth/token`
    - User must be authorized first
    - Post payload: { client_id, client_secret, code, redirect_uri }
    - Response: { access_token, scope, token_type }

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
