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


