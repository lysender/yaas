use axum::{Router, body::Body, middleware, response::Response};
use prost::Message;
use tokio::net::TcpListener;
use tracing::{error, info};

use crate::Result;
use crate::error::ErrorInfo;
use crate::state::AppState;
use crate::web::routes::all_routes;
use yaas::buffed::dto::ErrorMessageBuf;

pub async fn run_web_server(state: AppState) -> Result<()> {
    let port = state.config.server.port;

    let routes_all = Router::new()
        .merge(all_routes(state))
        .layer(middleware::map_response(response_mapper));

    // Setup the server
    // We will run behind a reverse proxy so we only bind to localhost
    let ip = "127.0.0.1";
    let addr = format!("{}:{}", ip, port);
    info!("HTTP server running on {}", addr);

    let listener = TcpListener::bind(addr).await.unwrap();
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

        let error_message = ErrorMessageBuf {
            status_code: e.status_code.as_u16() as u32,
            message: e.message.clone(),
            error: e.status_code.canonical_reason().unwrap().to_string(),
            error_code: e.error_code.clone(),
        };

        return Response::builder()
            .status(e.status_code)
            .header("Content-Type", "application/x-protobuf")
            .body(Body::from(error_message.encode_to_vec()))
            .unwrap();
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
