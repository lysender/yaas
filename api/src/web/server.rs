use axum::{
    Router, middleware,
    response::{IntoResponse, Response},
};
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::Result;
use crate::error::ErrorInfo;
use crate::state::AppState;
use crate::web::routes::all_routes;
use yaas::dto::ErrorMessageDto;

pub async fn run_web_server(state: AppState) -> Result<()> {
    let server_address = state.config.server.address.clone();

    let routes_all = Router::new()
        .merge(all_routes(state))
        .layer(middleware::map_response(response_mapper));

    // Setup the server
    // We will run behind a reverse proxy so we only bind to localhost
    info!("HTTP server running on {}", server_address);

    let listener = TcpListener::bind(server_address).await.unwrap();
    axum::serve(listener, routes_all.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    info!("HTTP server stopped");

    Ok(())
}

async fn response_mapper(res: Response) -> Response {
    let error = res.extensions().get::<ErrorInfo>();
    if let Some(e) = error {
        if e.status_code.is_server_error() {
            // Build the error response
            error!("{}", e.message);
        }

        let error_message = ErrorMessageDto {
            status_code: e.status_code.as_u16(),
            message: e.message.clone(),
            error: e.status_code.canonical_reason().unwrap().to_string(),
            error_code: e.error_code.clone(),
        };

        return (e.status_code, axum::Json(error_message)).into_response();
    }
    res
}

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}
