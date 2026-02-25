use std::sync::Arc;

use axum::extract::State;
use axum::http::header::CONTENT_TYPE;
use axum::http::StatusCode;
use axum::response::Html;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use tokio::net::TcpListener;
use tracing::error;

use crate::collector::NvmeCollector;
use crate::config::Config;
use crate::nvme::error::NvmeError;

#[derive(Clone)]
struct AppState {
    collector: Arc<NvmeCollector>,
}

pub async fn run_server(config: &Config, collector: Arc<NvmeCollector>) -> Result<(), NvmeError> {
    let app = Router::new()
        .route("/", get(root_handler))
        .route("/health", get(health_handler))
        .route("/metrics", get(metrics_handler))
        .with_state(AppState { collector });

    let listener = TcpListener::bind(config.listen_address)
        .await
        .map_err(|source| NvmeError::io_context("bind listen socket", source))?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .map_err(|source| NvmeError::io_context("http server", source))
}

async fn root_handler() -> impl IntoResponse {
    Html(
        "<html><body><h1>nvme-exporter</h1><ul><li><a href=\"/metrics\">/metrics</a></li><li><a href=\"/health\">/health</a></li></ul></body></html>",
    )
}

async fn health_handler() -> impl IntoResponse {
    "ok"
}

async fn metrics_handler(State(state): State<AppState>) -> impl IntoResponse {
    let collector = state.collector.clone();
    let result = tokio::task::spawn_blocking(move || collector.scrape()).await;

    match result {
        Ok(Ok(body)) => (
            [(CONTENT_TYPE, "text/plain; version=0.0.4; charset=utf-8")],
            body,
        )
            .into_response(),
        Ok(Err(error)) => {
            error!(error = %error, "scrape failed");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("scrape failed: {}", error),
            )
                .into_response()
        }
        Err(error) => {
            error!(error = %error, "scrape task join failure");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                "scrape task failed".to_string(),
            )
                .into_response()
        }
    }
}

async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };

    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                let _ = signal.recv().await;
            }
            Err(_) => {
                std::future::pending::<()>().await;
            }
        }
    };

    #[cfg(unix)]
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    #[cfg(not(unix))]
    ctrl_c.await;
}
