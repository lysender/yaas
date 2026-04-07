use serde::{Deserialize, Serialize};
use snafu::ensure;

use crate::Result;
use crate::dto::Paginated;
use crate::dto::{ListOrgAppsParamsDto, NewOrgAppDto, OrgAppDto, OrgAppSuggestionDto};
use crate::error::{CsrfTokenSnafu, ValidationSnafu};
use crate::run::AppState;
use crate::services::token::verify_csrf_token;

#[derive(Clone, Deserialize, Serialize)]
pub struct NewOrgAppFormData {
    pub token: String,
    pub app_id: String,
    pub app_name: String,
}

pub async fn list_org_apps_svc(
    state: &AppState,
    org_id: &str,
    params: ListOrgAppsParamsDto,
) -> Result<Paginated<OrgAppDto>> {
    state.db.org_apps.list(org_id.to_string(), params).await
}

pub async fn list_org_app_suggestions_svc(
    state: &AppState,
    org_id: &str,
    params: ListOrgAppsParamsDto,
) -> Result<Paginated<OrgAppSuggestionDto>> {
    state
        .db
        .org_apps
        .list_app_suggestions(org_id.to_string(), params)
        .await
}

pub async fn create_org_app_svc(
    state: &AppState,
    org_id: &str,
    data: NewOrgAppDto,
) -> Result<OrgAppDto> {
    // Ensure that the app exists
    let app_id = data.app_id.clone();
    let existing_app = state.db.apps.get(app_id.clone()).await?;

    ensure!(
        existing_app.is_some(),
        ValidationSnafu {
            msg: "App does not exist".to_string(),
        }
    );

    // Ensure that the app is not already linked to the org
    let existing_org_app = state
        .db
        .org_apps
        .find_app(org_id.to_string(), app_id)
        .await?;

    ensure!(
        existing_org_app.is_none(),
        ValidationSnafu {
            msg: "App is already linked to the organization".to_string(),
        }
    );

    state.db.org_apps.create(org_id.to_string(), data).await
}

pub async fn create_org_app_web_svc(
    state: &AppState,
    org_id: &str,
    form: NewOrgAppFormData,
) -> Result<OrgAppDto> {
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == "new_org_app", CsrfTokenSnafu);

    create_org_app_svc(
        state,
        org_id,
        NewOrgAppDto {
            app_id: form.app_id,
        },
    )
    .await
}

pub async fn get_org_app_svc(
    state: &AppState,
    org_id: &str,
    app_id: &str,
) -> Result<Option<OrgAppDto>> {
    state
        .db
        .org_apps
        .find_app(org_id.to_string(), app_id.to_string())
        .await
}

pub async fn delete_org_app_svc(state: &AppState, id: &str) -> Result<()> {
    state.db.org_apps.delete(id.to_string()).await
}

pub async fn delete_org_app_web_svc(
    state: &AppState,
    org_id: &str,
    app_id: &str,
    csrf_token: &str,
) -> Result<()> {
    let csrf_result = verify_csrf_token(csrf_token, &state.config.jwt_secret)?;
    ensure!(csrf_result == app_id, CsrfTokenSnafu);

    // Ensure the org_app actually belongs to the org
    let existing_org_app = get_org_app_svc(state, org_id, app_id).await?;

    ensure!(
        existing_org_app.is_some(),
        ValidationSnafu {
            msg: "App is not linked to the organization".to_string(),
        }
    );

    let existing_org_app = existing_org_app.expect("validated above");
    delete_org_app_svc(state, &existing_org_app.id).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::services::token::create_csrf_token_svc;
    use crate::test::TestCtx;

    use super::{
        NewOrgAppFormData, create_org_app_web_svc, delete_org_app_web_svc, get_org_app_svc,
    };

    #[tokio::test]
    async fn create_org_app_web_svc_creates_link_and_get_returns_it() {
        let ctx = TestCtx::new("org_apps_create_web").await.expect("test ctx");
        let fixture = ctx
            .seed_oauth_fixture(
                "Org Apps User",
                "org.apps.create@example.com",
                "password123",
                "Org Apps Org",
                "Org Apps App",
                "https://org-apps.example.com/callback",
                false,
            )
            .await
            .expect("oauth fixture");

        let csrf = create_csrf_token_svc("new_org_app", &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");

        let org_app = create_org_app_web_svc(
            &ctx.state,
            &fixture.auth.org.id,
            NewOrgAppFormData {
                token: csrf,
                app_id: fixture.app.id.clone(),
                app_name: fixture.app.name.clone(),
            },
        )
        .await
        .expect("org app should be created");

        let fetched = get_org_app_svc(&ctx.state, &fixture.auth.org.id, &fixture.app.id)
            .await
            .expect("query should pass")
            .expect("org app should exist");

        assert_eq!(org_app.id, fetched.id);
        assert_eq!(fetched.org_id, fixture.auth.org.id);
        assert_eq!(fetched.app_id, fixture.app.id);
    }

    #[tokio::test]
    async fn create_org_app_web_svc_rejects_invalid_csrf_token() {
        let ctx = TestCtx::new("org_apps_create_web_invalid_csrf")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_oauth_fixture(
                "Org Apps User",
                "org.apps.create.invalid@example.com",
                "password123",
                "Org Apps Org",
                "Org Apps App",
                "https://org-apps.example.com/callback",
                false,
            )
            .await
            .expect("oauth fixture");

        let result = create_org_app_web_svc(
            &ctx.state,
            &fixture.auth.org.id,
            NewOrgAppFormData {
                token: "invalid.token".to_string(),
                app_id: fixture.app.id.clone(),
                app_name: fixture.app.name,
            },
        )
        .await;

        assert!(result.is_err(), "invalid csrf should fail");
    }

    #[tokio::test]
    async fn delete_org_app_web_svc_deletes_link_and_get_returns_none() {
        let ctx = TestCtx::new("org_apps_delete_web").await.expect("test ctx");
        let fixture = ctx
            .seed_oauth_fixture(
                "Org Apps User",
                "org.apps.delete@example.com",
                "password123",
                "Org Apps Org",
                "Org Apps App",
                "https://org-apps.example.com/callback",
                false,
            )
            .await
            .expect("oauth fixture");

        let create_csrf = create_csrf_token_svc("new_org_app", &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");

        let _created = create_org_app_web_svc(
            &ctx.state,
            &fixture.auth.org.id,
            NewOrgAppFormData {
                token: create_csrf,
                app_id: fixture.app.id.clone(),
                app_name: fixture.app.name,
            },
        )
        .await
        .expect("org app should be created");

        let delete_csrf = create_csrf_token_svc(&fixture.app.id, &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");

        delete_org_app_web_svc(
            &ctx.state,
            &fixture.auth.org.id,
            &fixture.app.id,
            &delete_csrf,
        )
        .await
        .expect("org app should be deleted");

        let fetched = get_org_app_svc(&ctx.state, &fixture.auth.org.id, &fixture.app.id)
            .await
            .expect("query should pass");
        assert!(fetched.is_none());
    }

    #[tokio::test]
    async fn delete_org_app_web_svc_rejects_invalid_csrf_token() {
        let ctx = TestCtx::new("org_apps_delete_web_invalid_csrf")
            .await
            .expect("test ctx");
        let fixture = ctx
            .seed_oauth_fixture(
                "Org Apps User",
                "org.apps.delete.invalid@example.com",
                "password123",
                "Org Apps Org",
                "Org Apps App",
                "https://org-apps.example.com/callback",
                false,
            )
            .await
            .expect("oauth fixture");

        let create_csrf = create_csrf_token_svc("new_org_app", &ctx.state.config.jwt_secret)
            .expect("csrf token should be generated");

        let _created = create_org_app_web_svc(
            &ctx.state,
            &fixture.auth.org.id,
            NewOrgAppFormData {
                token: create_csrf,
                app_id: fixture.app.id.clone(),
                app_name: fixture.app.name,
            },
        )
        .await
        .expect("org app should be created");

        let result = delete_org_app_web_svc(
            &ctx.state,
            &fixture.auth.org.id,
            &fixture.app.id,
            "invalid.token",
        )
        .await;

        assert!(result.is_err(), "invalid csrf should fail");

        let fetched = get_org_app_svc(&ctx.state, &fixture.auth.org.id, &fixture.app.id)
            .await
            .expect("query should pass");
        assert!(fetched.is_some());
    }
}
