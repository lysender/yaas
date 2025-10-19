# Yaas API

Setup Endpoints:
- [x] POST `/setup`

Auth Endpoints:
- [x] POST `/auth/authorize`

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
