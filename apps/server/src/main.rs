use std::net::SocketAddr;

use axum::{routing::get, Json, Router};
use payq_types::HealthResponse;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    let app = Router::new()
        .route("/health", get(health))
        .route("/", get(health));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    tracing::info!("payq-server listening on http://{addr}");
    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .expect("bind address");

    axum::serve(listener, app).await.expect("server");
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        service: "payq-server".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}
