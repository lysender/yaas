# Yaas API

Setup Endpoints:
- [x] POST `/setup`

User Endpoints:
- [x] GET `/users`
- [x] POST `/users`
- [x] GET `/users/{user_id}`
- [x] PATCH `/users/{user_id}`
- [x] DELETE `/users/{user_id}`

App Endpoints:
- GET `/apps`
- POST `/apps`
- GET `/apps/{app_id}`
- PATCH `/apps/{app_id}`
- DELETE `/apps/{app_id}`

Org Endpoints:
- [x] GET `/orgs`
- [x] POST `/orgs`
- [x] GET `/orgs/{org_id}`
- [x] PATCH `/orgs/{org_id}`
- [x] DELETE `/orgs/{org_id}`

Org Member Endpoints:
- GET `/orgs/{org_id}/members`
- POST `/orgs/{org_id}/members`
- GET `/orgs/{org_id}/members/{org_member_id}`
- PATCH `/orgs/{org_id}/members/{org_member_id}`
- DELETE `/orgs/{org_id}/members/{org_member_id}`

Org App Endpoints:
- GET `/orgs/{org_id}/apps`
- POST `/orgs/{org_id}/apps`
- GET `/orgs/{org_id}/apps/{org_app_id}`
- DELETE `/orgs/{org_id}/apps/{org_app_id}`

Auth Endpoints:
- POST `/auth/authorize`
- GET `/auth/info`
