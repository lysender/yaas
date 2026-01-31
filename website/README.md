# Yaas Website

Frontend for yaas API.

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
- [ ] POST `/oauth/token`
    - Post payload: { client_id, client_secret, code, redirect_uri }
    - Response: { access_token, scope, token_type }

