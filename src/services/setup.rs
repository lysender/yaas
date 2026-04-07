use snafu::ensure;

use crate::dto::{NewPasswordDto, NewUserDto, SetupBodyDto, SuperuserDto};
use crate::error::ValidationSnafu;
use crate::services::password::hash_password;
use crate::{Result, run::AppState};

pub async fn setup_superuser_svc(state: &AppState, payload: SetupBodyDto) -> Result<SuperuserDto> {
    // Validate setup key
    ensure!(
        Some(payload.setup_key) == state.config.superuser.setup_key,
        ValidationSnafu {
            msg: "Invalid setup key".to_string(),
        }
    );

    // Make sure there are no superusers yet
    let superusers = state.db.superusers.list().await?;
    ensure!(
        superusers.is_empty(),
        ValidationSnafu {
            msg: "Superuser already exists".to_string(),
        }
    );

    let new_user = NewUserDto {
        email: payload.email,
        name: "Superuser".to_string(),
    };

    let new_password = NewPasswordDto {
        password: hash_password(&payload.password)?,
    };

    let superuser = state.db.superusers.setup(new_user, new_password).await?;

    Ok(superuser)
}

pub async fn setup_status_svc(state: &AppState) -> Result<bool> {
    let superusers = state.db.superusers.list().await?;
    Ok(!superusers.is_empty())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::dto::SetupBodyDto;
    use crate::test::TestCtx;

    use super::{setup_status_svc, setup_superuser_svc};

    const TEST_SETUP_KEY: &str = "12345678-1234-1234-1234-123456789012";

    fn set_setup_key(ctx: &mut TestCtx, setup_key: &str) {
        let mut config = (*ctx.state.config).clone();
        config.superuser.setup_key = Some(setup_key.to_string());
        ctx.state.config = Arc::new(config);
    }

    #[tokio::test]
    async fn setup_superuser_svc_creates_superuser_successfully() {
        let mut ctx = TestCtx::new("setup_superuser_success")
            .await
            .expect("test ctx");
        set_setup_key(&mut ctx, TEST_SETUP_KEY);

        let superuser = setup_superuser_svc(
            &ctx.state,
            SetupBodyDto {
                setup_key: TEST_SETUP_KEY.to_string(),
                email: "root@example.com".to_string(),
                password: "password123".to_string(),
            },
        )
        .await
        .expect("setup should succeed");

        assert!(!superuser.id.is_empty());

        let status = setup_status_svc(&ctx.state)
            .await
            .expect("status query should pass");
        assert!(status);
    }

    #[tokio::test]
    async fn setup_superuser_svc_rejects_invalid_setup_key() {
        let mut ctx = TestCtx::new("setup_superuser_invalid_key")
            .await
            .expect("test ctx");
        set_setup_key(&mut ctx, TEST_SETUP_KEY);

        let result = setup_superuser_svc(
            &ctx.state,
            SetupBodyDto {
                setup_key: "00000000-0000-0000-0000-000000000000".to_string(),
                email: "root@example.com".to_string(),
                password: "password123".to_string(),
            },
        )
        .await;

        assert!(result.is_err(), "invalid setup key should fail");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "Invalid setup key");
    }

    #[tokio::test]
    async fn setup_superuser_svc_rejects_when_setup_is_already_done() {
        let mut ctx = TestCtx::new("setup_superuser_already_done")
            .await
            .expect("test ctx");
        set_setup_key(&mut ctx, TEST_SETUP_KEY);

        setup_superuser_svc(
            &ctx.state,
            SetupBodyDto {
                setup_key: TEST_SETUP_KEY.to_string(),
                email: "root@example.com".to_string(),
                password: "password123".to_string(),
            },
        )
        .await
        .expect("first setup should succeed");

        let result = setup_superuser_svc(
            &ctx.state,
            SetupBodyDto {
                setup_key: TEST_SETUP_KEY.to_string(),
                email: "root2@example.com".to_string(),
                password: "password123".to_string(),
            },
        )
        .await;

        assert!(result.is_err(), "second setup should fail");
        let err = result.err().expect("error should exist");
        assert_eq!(err.to_string(), "Superuser already exists");
    }

    #[tokio::test]
    async fn setup_status_svc_returns_false_when_no_superusers_exist() {
        let ctx = TestCtx::new("setup_status_false").await.expect("test ctx");

        let status = setup_status_svc(&ctx.state)
            .await
            .expect("status query should pass");

        assert!(!status);
    }

    #[tokio::test]
    async fn setup_status_svc_returns_true_when_superuser_exists() {
        let ctx = TestCtx::new("setup_status_true").await.expect("test ctx");
        let user = ctx
            .seed_user_with_password("Super User", "super.status@example.com", "password123")
            .await
            .expect("seed user");

        ctx.state
            .db
            .superusers
            .create(user.id)
            .await
            .expect("superuser should be created");

        let status = setup_status_svc(&ctx.state)
            .await
            .expect("status query should pass");

        assert!(status);
    }
}
