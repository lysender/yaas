# Yaas API

User Endpoints:
- GET `/users`
- POST `/users`
- GET `/users/{user_id}`
- PATCH `/users/{user_id}`
- DELETE `/users/{user_id}`

App Endpoints:
- GET `/apps`
- POST `/apps`
- GET `/apps/{app_id}`
- PATCH `/apps/{app_id}`
- DELETE `/apps/{app_id}`

Org Endpoints:
- GET `/orgs`
- POST `/orgs`
- GET `/orgs/{org_id}`
- PATCH `/orgs/{org_id}`
- DELETE `/orgs/{org_id}`

Org Member Endpoints:
- GET `/orgs/{org_id}/members`
- POST `/orgs/{org_id}/members`
- GET `/orgs/{org_id}/members/{org_member_id}`
- PATCH `/orgs/{org_id}/members/{org_member_id}`
- DELETE `/orgs/{org_id}/members/{org_member_id}`

Org App Endpoints:
- GET `/orgs/{org_id}/apps`
- POST `/orgs/{org_id}/apps`

Oauth Endpoints:
- POST `/oauth/authorize`
- GET `/oauth/info`
