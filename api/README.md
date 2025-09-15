# Yaas API

Setup Endpoints:
- [x] POST `/setup`

Auth Endpoints:
- [x] POST `/auth/authorize`

User Endpoints:
- [x] GET `/user`
- [x] GET `/user/authz`
- [x] POST `/user/change-password`

Users Endpoints:
- [x] GET `/users`
- [x] POST `/users`
- [x] GET `/users/{user_id}`
- [x] PATCH `/users/{user_id}`
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
- [ ] GET `/orgs/{org_id}/members`
- [ ] jPOST `/orgs/{org_id}/members`
- [ ] GET `/orgs/{org_id}/members/{org_member_id}`
- [ ] PATCH `/orgs/{org_id}/members/{org_member_id}`
- [ ] DELETE `/orgs/{org_id}/members/{org_member_id}`

Org Apps Endpoints:
- [ ] GET `/orgs/{org_id}/apps`
- [ ] POST `/orgs/{org_id}/apps`
- [ ] GET `/orgs/{org_id}/apps/{org_app_id}`
- [ ] DELETE `/orgs/{org_id}/apps/{org_app_id}`
