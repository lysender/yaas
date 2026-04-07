use serde::{Deserialize, Serialize};
use snafu::ensure;

use crate::dto::Paginated;
use crate::dto::{AppDto, ListAppsParamsDto, NewAppDto, UpdateAppDto};
use crate::error::CsrfTokenSnafu;
use crate::run::AppState;
use crate::services::token::verify_csrf_token;
use crate::{Error, Result};

#[derive(Clone, Deserialize, Serialize)]
pub struct NewAppFormData {
    pub name: String,
    pub redirect_uri: String,
    pub token: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct UpdateAppFormData {
    pub token: String,
    pub name: String,
    pub redirect_uri: String,
}

pub async fn list_apps_svc(
    state: &AppState,
    params: ListAppsParamsDto,
) -> Result<Paginated<AppDto>> {
    state.db.apps.list(params).await
}

pub async fn create_app_svc(state: &AppState, data: NewAppDto) -> Result<AppDto> {
    state.db.apps.create(data).await
}

pub async fn create_app_web_svc(state: &AppState, form: NewAppFormData) -> Result<AppDto> {
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == "new_app", CsrfTokenSnafu);

    create_app_svc(
        state,
        NewAppDto {
            name: form.name,
            redirect_uri: form.redirect_uri,
        },
    )
    .await
}

pub async fn get_app_svc(state: &AppState, id: &str) -> Result<Option<AppDto>> {
    state.db.apps.get(id.to_string()).await
}

pub async fn update_app_svc(state: &AppState, id: &str, data: UpdateAppDto) -> Result<bool> {
    state.db.apps.update(id.to_string(), data).await
}

pub async fn update_app_web_svc(
    state: &AppState,
    app_id: &str,
    form: UpdateAppFormData,
) -> Result<AppDto> {
    let csrf_result = verify_csrf_token(&form.token, &state.config.jwt_secret)?;
    ensure!(csrf_result == app_id, CsrfTokenSnafu);

    update_app_svc(
        state,
        app_id,
        UpdateAppDto {
            name: Some(form.name),
            redirect_uri: Some(form.redirect_uri),
        },
    )
    .await?;

    // Fetch the updated app to return
    let Some(updated_app) = get_app_svc(state, app_id).await? else {
        return Err(Error::AppNotFound);
    };

    Ok(updated_app)
}

pub async fn regenerate_app_secret_svc(state: &AppState, id: &str) -> Result<bool> {
    state.db.apps.regenerate_secret(id.to_string()).await
}

pub async fn regenerate_app_secret_web_svc(
    state: &AppState,
    app_id: &str,
    csrf_token: &str,
) -> Result<()> {
    let csrf_result = verify_csrf_token(csrf_token, &state.config.jwt_secret)?;
    ensure!(csrf_result == app_id, CsrfTokenSnafu);

    regenerate_app_secret_svc(state, app_id).await?;

    Ok(())
}

pub async fn delete_app_svc(state: &AppState, id: &str) -> Result<bool> {
    state.db.apps.delete(id.to_string()).await
}

pub async fn delete_app_web_svc(state: &AppState, app_id: &str, csrf_token: &str) -> Result<()> {
    let csrf_result = verify_csrf_token(csrf_token, &state.config.jwt_secret)?;
    ensure!(csrf_result == app_id, CsrfTokenSnafu);

    delete_app_svc(state, app_id).await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::dto::{ListAppsParamsDto, NewAppDto, UpdateAppDto};
    use crate::test::TestCtx;

    use super::{
        create_app_svc, delete_app_svc, get_app_svc, list_apps_svc, regenerate_app_secret_svc,
        update_app_svc,
    };

    #[tokio::test]
    async fn create_app_creates_new_app() {
        let ctx = TestCtx::new("apps_create").await.expect("test ctx");

        let app = create_app_svc(
            &ctx.state,
            NewAppDto {
                name: "Photos".to_string(),
                redirect_uri: "https://photos.example.com/oauth/callback".to_string(),
            },
        )
        .await
        .expect("app should be created");

        assert!(!app.id.is_empty());
        assert_eq!(app.name, "Photos");
        assert_eq!(
            app.redirect_uri,
            "https://photos.example.com/oauth/callback"
        );
        assert!(!app.client_id.is_empty());
        assert!(!app.client_secret.is_empty());
    }

    #[tokio::test]
    async fn get_app_returns_existing_app() {
        let ctx = TestCtx::new("apps_get_existing").await.expect("test ctx");
        let seeded = ctx
            .seed_app("Gallery", "https://gallery.example.com/oauth/callback")
            .await
            .expect("seed app");

        let app = get_app_svc(&ctx.state, &seeded.id)
            .await
            .expect("query should pass")
            .expect("app should exist");

        assert_eq!(app.id, seeded.id);
        assert_eq!(app.name, "Gallery");
    }

    #[tokio::test]
    async fn get_app_returns_none_for_non_existing_app() {
        let ctx = TestCtx::new("apps_get_missing").await.expect("test ctx");

        let app = get_app_svc(&ctx.state, "app_non_existing")
            .await
            .expect("query should pass");

        assert!(app.is_none());
    }

    #[tokio::test]
    async fn list_apps_returns_created_apps() {
        let ctx = TestCtx::new("apps_list").await.expect("test ctx");
        ctx.seed_app("App One", "https://one.example.com/oauth/callback")
            .await
            .expect("seed app one");
        ctx.seed_app("App Two", "https://two.example.com/oauth/callback")
            .await
            .expect("seed app two");

        let apps = list_apps_svc(
            &ctx.state,
            ListAppsParamsDto {
                page: Some(1),
                per_page: Some(10),
                keyword: None,
            },
        )
        .await
        .expect("list should pass");

        assert_eq!(apps.meta.total_records, 2);
        assert_eq!(apps.data.len(), 2);
    }

    #[tokio::test]
    async fn update_app_updates_name_and_redirect_uri() {
        let ctx = TestCtx::new("apps_update").await.expect("test ctx");
        let app = ctx
            .seed_app("Calendar", "https://calendar.example.com/oauth/callback")
            .await
            .expect("seed app");

        let updated = update_app_svc(
            &ctx.state,
            &app.id,
            UpdateAppDto {
                name: Some("Calendar Pro".to_string()),
                redirect_uri: Some("https://calendar.example.com/oauth/new-callback".to_string()),
            },
        )
        .await
        .expect("update should pass");

        assert!(updated);

        let reloaded = get_app_svc(&ctx.state, &app.id)
            .await
            .expect("get should pass")
            .expect("app should exist");

        assert_eq!(reloaded.name, "Calendar Pro");
        assert_eq!(
            reloaded.redirect_uri,
            "https://calendar.example.com/oauth/new-callback"
        );
    }

    #[tokio::test]
    async fn update_app_returns_false_for_non_existing_app() {
        let ctx = TestCtx::new("apps_update_missing").await.expect("test ctx");

        let updated = update_app_svc(
            &ctx.state,
            "app_non_existing",
            UpdateAppDto {
                name: Some("Nope".to_string()),
                redirect_uri: Some("https://none.example.com/callback".to_string()),
            },
        )
        .await
        .expect("update should pass");

        assert!(!updated);
    }

    #[tokio::test]
    async fn regenerate_app_secret_rotates_credentials() {
        let ctx = TestCtx::new("apps_regenerate_secret")
            .await
            .expect("test ctx");
        let app = ctx
            .seed_app("Drive", "https://drive.example.com/oauth/callback")
            .await
            .expect("seed app");

        let old_client_id = app.client_id.clone();
        let old_client_secret = app.client_secret.clone();

        let regenerated = regenerate_app_secret_svc(&ctx.state, &app.id)
            .await
            .expect("regenerate should pass");
        assert!(regenerated);

        let reloaded = get_app_svc(&ctx.state, &app.id)
            .await
            .expect("get should pass")
            .expect("app should exist");

        assert_ne!(reloaded.client_id, old_client_id);
        assert_ne!(reloaded.client_secret, old_client_secret);
    }

    #[tokio::test]
    async fn delete_app_marks_app_as_deleted() {
        let ctx = TestCtx::new("apps_delete").await.expect("test ctx");
        let app = ctx
            .seed_app("Delete Me", "https://delete.example.com/oauth/callback")
            .await
            .expect("seed app");

        let deleted = delete_app_svc(&ctx.state, &app.id)
            .await
            .expect("delete should pass");
        assert!(deleted);

        let reloaded = get_app_svc(&ctx.state, &app.id)
            .await
            .expect("get should pass");
        assert!(reloaded.is_none());
    }
}
