use snafu::{OptionExt, ensure};

use crate::ctx::Ctx;
use crate::dto::{
    ActorPayloadDto, NewOauthCodeDto, OauthAuthorizationCodeDto, OauthAuthorizeDto,
    OauthClientAppDto, OauthClientLookupDto, OauthTokenRequestDto, OauthTokenResponseDto, Scope,
    to_scopes,
};
use crate::error::{
    AppNotRegisteredSnafu, ForbiddenSnafu, InvalidClientSnafu, OauthCodeInvalidSnafu,
    OauthInvalidScopesSnafu, OauthStateMismatchSnafu, RedirectUriMistmatchSnafu,
};
use crate::run::AppState;
use crate::services::oauth_code::{create_oauth_code_svc, delete_oauth_code_svc};
use crate::services::token::create_auth_token;
use crate::utils::{IdPrefix, generate_id, validate_redirect_uri};
use crate::{Error, Result};

pub async fn create_authorization_code_svc(
    state: &AppState,
    ctx: &Ctx,
    query: &OauthAuthorizeDto,
) -> Result<OauthAuthorizationCodeDto> {
    // Validate scopes
    let scope_list: Vec<String> = query
        .scope
        .split(' ')
        .filter(|scope| !scope.is_empty())
        .map(|scope| scope.to_string())
        .collect();

    // Convert it to scope enums
    let scopes = to_scopes(&scope_list)?;

    // Allowed scopes
    let allowed_scopes = [Scope::Auth, Scope::Oauth];

    // Ensure all requested scopes are allowed
    let invalid_scopes = scopes
        .iter()
        .filter(|s| !allowed_scopes.contains(s))
        .count();

    ensure!(invalid_scopes == 0, InvalidClientSnafu);

    let actor = ctx.actor().context(InvalidClientSnafu)?;

    // Validate client_id from the apps table
    let app = state
        .db
        .apps
        .find_by_client_id(query.client_id.clone())
        .await?;

    let app = app.context(InvalidClientSnafu)?;
    let app_id = app.id.clone();
    let actor_org_id = actor.org_id.clone();
    let actor_user_id = actor.id.clone();

    // Ensure redirect_uri is valid and matches the registered one
    ensure!(
        validate_redirect_uri(&app.redirect_uri, &query.redirect_uri),
        RedirectUriMistmatchSnafu
    );

    // Ensure that the app is registered to the user's current org
    let org_app = state
        .db
        .org_apps
        .find_app(actor_org_id.clone(), app_id.clone())
        .await?;

    ensure!(org_app.is_some(), AppNotRegisteredSnafu);

    // Generate oauth_code object to be finalized later at token generation
    let code = generate_id(IdPrefix::OauthCode);

    let new_code = NewOauthCodeDto {
        code: code.clone(),
        state: query.state.clone(),
        redirect_uri: query.redirect_uri.clone(),
        scope: query.scope.clone(),
        app_id,
        org_id: actor_org_id,
        user_id: actor_user_id,
    };

    create_oauth_code_svc(&state, new_code).await?;

    let auth_code = OauthAuthorizationCodeDto {
        code: code.clone(),
        state: query.state.clone(),
    };

    Ok(auth_code)
}

pub async fn exchange_code_for_access_token_svc(
    state: &AppState,
    payload: &OauthTokenRequestDto,
) -> Result<OauthTokenResponseDto> {
    // Find the authorization code
    let oauth_code = state.db.oauth_codes.find_by_code(&payload.code).await?;

    let oauth_code = oauth_code.context(OauthCodeInvalidSnafu)?;
    let oauth_org_id = oauth_code.org_id.clone();
    let oauth_user_id = oauth_code.user_id.clone();

    // Ensure that parameters match those used during authorization
    ensure!(oauth_code.state == payload.state, OauthStateMismatchSnafu);
    ensure!(
        oauth_code.redirect_uri == payload.redirect_uri,
        RedirectUriMistmatchSnafu
    );

    // Validate client_id and client_secret
    let app = state
        .db
        .apps
        .find_by_client_id(payload.client_id.clone())
        .await?;

    let app = app.context(InvalidClientSnafu)?;

    ensure!(
        app.client_secret == payload.client_secret,
        InvalidClientSnafu
    );

    // Parse scopes
    let scope_list: Vec<String> = oauth_code
        .scope
        .split(' ')
        .filter(|scope| !scope.is_empty())
        .map(|scope| scope.to_string())
        .collect();

    let scopes = to_scopes(&scope_list)?;

    // Allowed scopes
    let allowed_scopes = [Scope::Auth, Scope::Oauth];

    // Ensure all requested scopes are allowed
    let invalid_scopes: Vec<&Scope> = scopes
        .iter()
        .filter(|s| !allowed_scopes.contains(s))
        .collect();

    ensure!(invalid_scopes.is_empty(), OauthInvalidScopesSnafu);

    // Fetch roles for the user in the org
    let membership = state
        .db
        .org_members
        .find_member(oauth_org_id.clone(), oauth_user_id.clone())
        .await?;

    let membership = membership.context(ForbiddenSnafu {
        msg: "User must be a member of the org".to_string(),
    })?;

    // Count org memberships for the user
    let org_count = state
        .db
        .org_members
        .list_memberships_count(oauth_user_id.clone())
        .await?;

    // Create a token
    let payload = ActorPayloadDto {
        id: oauth_user_id,
        org_id: oauth_org_id,
        org_count: org_count as i32,
        roles: membership.roles.clone(),
        scopes,
    };

    let token = create_auth_token(&payload, &state.config.jwt_secret)?;

    // Cleanup oauth code so it cannot be used again
    delete_oauth_code_svc(&state, &oauth_code.id).await?;

    let response = OauthTokenResponseDto {
        access_token: token,
        scope: oauth_code.scope,
        token_type: "app".to_string(),
    };

    Ok(response)
}

pub async fn lookup_oauth_client_app_svc(
    state: &AppState,
    payload: &OauthClientLookupDto,
) -> Result<OauthClientAppDto> {
    // Find app by client_id
    let Some(app) = state
        .db
        .apps
        .find_by_client_id(payload.client_id.to_owned())
        .await?
    else {
        return Err(Error::InvalidClient);
    };

    // Validate if redirect_uri is valid
    ensure!(
        validate_redirect_uri(&app.redirect_uri, &payload.redirect_uri),
        InvalidClientSnafu
    );

    Ok(OauthClientAppDto { name: app.name })
}

#[cfg(test)]
mod tests {
    use crate::dto::{NewOauthCodeDto, OauthAuthorizeDto, OauthTokenRequestDto, Scope};
    use crate::test::TestCtx;
    use crate::utils::{IdPrefix, generate_id};

    use super::{create_authorization_code_svc, exchange_code_for_access_token_svc};

    fn build_authorize(client_id: String, redirect_uri: &str, scope: &str) -> OauthAuthorizeDto {
        OauthAuthorizeDto {
            client_id,
            redirect_uri: redirect_uri.to_string(),
            scope: scope.to_string(),
            state: "state-1".to_string(),
        }
    }

    fn build_token_request(
        client_id: String,
        client_secret: String,
        code: String,
        state: &str,
        redirect_uri: &str,
    ) -> OauthTokenRequestDto {
        OauthTokenRequestDto {
            client_id,
            client_secret,
            code,
            state: state.to_string(),
            redirect_uri: redirect_uri.to_string(),
        }
    }

    #[tokio::test]
    async fn create_authorization_code_happy_path() {
        let ctx = TestCtx::new("oauth_create_code_happy")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_oauth_fixture(
                "OAuth User",
                "oauth.create@example.com",
                "password123",
                "OAuth Org",
                "OAuth App",
                "https://oauth.example.com/callback",
                true,
            )
            .await
            .expect("oauth fixture");

        let actor_ctx = fixture.auth.to_ctx(vec![Scope::Auth]);
        let query = build_authorize(
            fixture.app.client_id.clone(),
            "https://oauth.example.com/callback",
            "auth oauth",
        );

        let code = create_authorization_code_svc(&ctx.state, &actor_ctx, &query)
            .await
            .expect("authorization code should be created");

        assert!(!code.code.is_empty());
        assert_eq!(code.state, "state-1");
    }

    #[tokio::test]
    async fn create_authorization_code_rejects_invalid_scope() {
        let ctx = TestCtx::new("oauth_create_code_invalid_scope")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_oauth_fixture(
                "OAuth User",
                "oauth.bad.scope@example.com",
                "password123",
                "OAuth Org",
                "OAuth App",
                "https://oauth.example.com/callback",
                true,
            )
            .await
            .expect("oauth fixture");

        let actor_ctx = fixture.auth.to_ctx(vec![Scope::Auth]);
        let query = build_authorize(
            fixture.app.client_id.clone(),
            "https://oauth.example.com/callback",
            "invalid_scope",
        );

        let result = create_authorization_code_svc(&ctx.state, &actor_ctx, &query).await;
        assert!(result.is_err(), "should reject invalid scope");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "Invalid scopes: invalid_scope");
    }

    #[tokio::test]
    async fn create_authorization_code_rejects_missing_client_app() {
        let ctx = TestCtx::new("oauth_create_code_missing_app")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_auth_fixture(
                "OAuth User",
                "oauth.noapp@example.com",
                "password123",
                "OAuth Org",
            )
            .await
            .expect("auth fixture");

        let actor_ctx = fixture.to_ctx(vec![Scope::Auth]);
        let query = build_authorize(
            "11111111-1111-1111-1111-111111111111".to_string(),
            "https://oauth.example.com/callback",
            "auth",
        );

        let result = create_authorization_code_svc(&ctx.state, &actor_ctx, &query).await;
        assert!(result.is_err(), "should reject missing app");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "Invalid client");
    }

    #[tokio::test]
    async fn create_authorization_code_rejects_redirect_uri_mismatch() {
        let ctx = TestCtx::new("oauth_create_code_redirect_mismatch")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_oauth_fixture(
                "OAuth User",
                "oauth.redirect@example.com",
                "password123",
                "OAuth Org",
                "OAuth App",
                "https://oauth.example.com/callback",
                true,
            )
            .await
            .expect("oauth fixture");

        let actor_ctx = fixture.auth.to_ctx(vec![Scope::Auth]);
        let query = build_authorize(
            fixture.app.client_id.clone(),
            "https://attacker.example.com/callback",
            "auth",
        );

        let result = create_authorization_code_svc(&ctx.state, &actor_ctx, &query).await;
        assert!(result.is_err(), "should reject redirect mismatch");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "OAuth redirect_uri mismatch");
    }

    #[tokio::test]
    async fn create_authorization_code_rejects_app_not_registered_in_org() {
        let ctx = TestCtx::new("oauth_create_code_not_registered")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_oauth_fixture(
                "OAuth User",
                "oauth.not.registered@example.com",
                "password123",
                "OAuth Org",
                "OAuth App",
                "https://oauth.example.com/callback",
                false,
            )
            .await
            .expect("oauth fixture");

        let actor_ctx = fixture.auth.to_ctx(vec![Scope::Auth]);
        let query = build_authorize(
            fixture.app.client_id.clone(),
            "https://oauth.example.com/callback",
            "auth",
        );

        let result = create_authorization_code_svc(&ctx.state, &actor_ctx, &query).await;
        assert!(result.is_err(), "should reject unregistered app");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "OAuth app not registered in the org");
    }

    #[tokio::test]
    async fn exchange_code_for_access_token_happy_path() {
        let ctx = TestCtx::new("oauth_exchange_happy")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_oauth_fixture(
                "OAuth User",
                "oauth.exchange@example.com",
                "password123",
                "OAuth Org",
                "OAuth App",
                "https://oauth.example.com/callback",
                true,
            )
            .await
            .expect("oauth fixture");

        let actor_ctx = fixture.auth.to_ctx(vec![Scope::Auth]);
        let authorize = build_authorize(
            fixture.app.client_id.clone(),
            "https://oauth.example.com/callback",
            "auth oauth",
        );

        let code = create_authorization_code_svc(&ctx.state, &actor_ctx, &authorize)
            .await
            .expect("authorization code should be created");

        let payload = build_token_request(
            fixture.app.client_id.clone(),
            fixture.app.client_secret.clone(),
            code.code,
            &code.state,
            "https://oauth.example.com/callback",
        );

        let token = exchange_code_for_access_token_svc(&ctx.state, &payload)
            .await
            .expect("token exchange should succeed");

        assert!(!token.access_token.is_empty());
        assert_eq!(token.scope, "auth oauth");
        assert_eq!(token.token_type, "app");
    }

    #[tokio::test]
    async fn exchange_code_for_access_token_rejects_code_not_found() {
        let ctx = TestCtx::new("oauth_exchange_missing_code")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_oauth_fixture(
                "OAuth User",
                "oauth.missing.code@example.com",
                "password123",
                "OAuth Org",
                "OAuth App",
                "https://oauth.example.com/callback",
                true,
            )
            .await
            .expect("oauth fixture");

        let payload = build_token_request(
            fixture.app.client_id,
            fixture.app.client_secret,
            "22222222-2222-2222-2222-222222222222".to_string(),
            "state-1",
            "https://oauth.example.com/callback",
        );

        let result = exchange_code_for_access_token_svc(&ctx.state, &payload).await;
        assert!(result.is_err(), "should reject missing code");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "OAuth code invalid");
    }

    #[tokio::test]
    async fn exchange_code_for_access_token_rejects_state_mismatch() {
        let ctx = TestCtx::new("oauth_exchange_state_mismatch")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_oauth_fixture(
                "OAuth User",
                "oauth.state.mismatch@example.com",
                "password123",
                "OAuth Org",
                "OAuth App",
                "https://oauth.example.com/callback",
                true,
            )
            .await
            .expect("oauth fixture");

        let actor_ctx = fixture.auth.to_ctx(vec![Scope::Auth]);
        let authorize = build_authorize(
            fixture.app.client_id.clone(),
            "https://oauth.example.com/callback",
            "auth",
        );
        let code = create_authorization_code_svc(&ctx.state, &actor_ctx, &authorize)
            .await
            .expect("authorization code should be created");

        let payload = build_token_request(
            fixture.app.client_id,
            fixture.app.client_secret,
            code.code,
            "different-state",
            "https://oauth.example.com/callback",
        );

        let result = exchange_code_for_access_token_svc(&ctx.state, &payload).await;
        assert!(result.is_err(), "should reject state mismatch");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "OAuth state mismatch");
    }

    #[tokio::test]
    async fn exchange_code_for_access_token_rejects_redirect_uri_mismatch() {
        let ctx = TestCtx::new("oauth_exchange_redirect_mismatch")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_oauth_fixture(
                "OAuth User",
                "oauth.exchange.redirect@example.com",
                "password123",
                "OAuth Org",
                "OAuth App",
                "https://oauth.example.com/callback",
                true,
            )
            .await
            .expect("oauth fixture");

        let actor_ctx = fixture.auth.to_ctx(vec![Scope::Auth]);
        let authorize = build_authorize(
            fixture.app.client_id.clone(),
            "https://oauth.example.com/callback",
            "auth",
        );
        let code = create_authorization_code_svc(&ctx.state, &actor_ctx, &authorize)
            .await
            .expect("authorization code should be created");

        let payload = build_token_request(
            fixture.app.client_id,
            fixture.app.client_secret,
            code.code,
            &code.state,
            "https://other.example.com/callback",
        );

        let result = exchange_code_for_access_token_svc(&ctx.state, &payload).await;
        assert!(result.is_err(), "should reject redirect mismatch");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "OAuth redirect_uri mismatch");
    }

    #[tokio::test]
    async fn exchange_code_for_access_token_rejects_missing_client_app() {
        let ctx = TestCtx::new("oauth_exchange_missing_client")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_oauth_fixture(
                "OAuth User",
                "oauth.exchange.no.client@example.com",
                "password123",
                "OAuth Org",
                "OAuth App",
                "https://oauth.example.com/callback",
                true,
            )
            .await
            .expect("oauth fixture");

        let actor_ctx = fixture.auth.to_ctx(vec![Scope::Auth]);
        let authorize = build_authorize(
            fixture.app.client_id.clone(),
            "https://oauth.example.com/callback",
            "auth",
        );
        let code = create_authorization_code_svc(&ctx.state, &actor_ctx, &authorize)
            .await
            .expect("authorization code should be created");

        let payload = build_token_request(
            "33333333-3333-3333-3333-333333333333".to_string(),
            fixture.app.client_secret,
            code.code,
            &code.state,
            "https://oauth.example.com/callback",
        );

        let result = exchange_code_for_access_token_svc(&ctx.state, &payload).await;
        assert!(result.is_err(), "should reject missing client app");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "Invalid client");
    }

    #[tokio::test]
    async fn exchange_code_for_access_token_rejects_invalid_client_secret() {
        let ctx = TestCtx::new("oauth_exchange_invalid_secret")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_oauth_fixture(
                "OAuth User",
                "oauth.exchange.bad.secret@example.com",
                "password123",
                "OAuth Org",
                "OAuth App",
                "https://oauth.example.com/callback",
                true,
            )
            .await
            .expect("oauth fixture");

        let actor_ctx = fixture.auth.to_ctx(vec![Scope::Auth]);
        let authorize = build_authorize(
            fixture.app.client_id.clone(),
            "https://oauth.example.com/callback",
            "auth",
        );
        let code = create_authorization_code_svc(&ctx.state, &actor_ctx, &authorize)
            .await
            .expect("authorization code should be created");

        let payload = build_token_request(
            fixture.app.client_id,
            "44444444-4444-4444-4444-444444444444".to_string(),
            code.code,
            &code.state,
            "https://oauth.example.com/callback",
        );

        let result = exchange_code_for_access_token_svc(&ctx.state, &payload).await;
        assert!(result.is_err(), "should reject invalid client secret");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "Invalid client");
    }

    #[tokio::test]
    async fn exchange_code_for_access_token_rejects_invalid_scope() {
        let ctx = TestCtx::new("oauth_exchange_invalid_scope")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_oauth_fixture(
                "OAuth User",
                "oauth.exchange.bad.scope@example.com",
                "password123",
                "OAuth Org",
                "OAuth App",
                "https://oauth.example.com/callback",
                true,
            )
            .await
            .expect("oauth fixture");

        let oauth_code = ctx
            .state
            .db
            .oauth_codes
            .create(NewOauthCodeDto {
                code: generate_id(IdPrefix::OauthCode),
                state: "state-1".to_string(),
                redirect_uri: "https://oauth.example.com/callback".to_string(),
                scope: "vault".to_string(),
                app_id: fixture.app.id.clone(),
                org_id: fixture.auth.org.id.clone(),
                user_id: fixture.auth.user.id.clone(),
            })
            .await
            .expect("oauth code should be inserted");

        let payload = build_token_request(
            fixture.app.client_id,
            fixture.app.client_secret,
            oauth_code.code,
            "state-1",
            "https://oauth.example.com/callback",
        );

        let result = exchange_code_for_access_token_svc(&ctx.state, &payload).await;
        assert!(result.is_err(), "should reject invalid scope");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "OAuth scopes invalid");
    }

    #[tokio::test]
    async fn exchange_code_for_access_token_rejects_user_not_member_of_org() {
        let ctx = TestCtx::new("oauth_exchange_user_not_member")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_oauth_fixture(
                "OAuth User",
                "oauth.exchange.not.member@example.com",
                "password123",
                "OAuth Org",
                "OAuth App",
                "https://oauth.example.com/callback",
                true,
            )
            .await
            .expect("oauth fixture");

        let actor_ctx = fixture.auth.to_ctx(vec![Scope::Auth]);
        let authorize = build_authorize(
            fixture.app.client_id.clone(),
            "https://oauth.example.com/callback",
            "auth",
        );
        let code = create_authorization_code_svc(&ctx.state, &actor_ctx, &authorize)
            .await
            .expect("authorization code should be created");

        let member = ctx
            .state
            .db
            .org_members
            .find_member(fixture.auth.org.id.clone(), fixture.auth.user.id.clone())
            .await
            .expect("membership query should pass")
            .expect("membership should exist");
        ctx.state
            .db
            .org_members
            .delete(member.id)
            .await
            .expect("membership should be removable");

        let payload = build_token_request(
            fixture.app.client_id,
            fixture.app.client_secret,
            code.code,
            &code.state,
            "https://oauth.example.com/callback",
        );

        let result = exchange_code_for_access_token_svc(&ctx.state, &payload).await;
        assert!(result.is_err(), "should reject non-member user");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "User must be a member of the org");
    }
}
