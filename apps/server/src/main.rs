mod auction;
mod intent;
mod mcp;
mod routes;
mod solana;
mod state;

use std::{net::SocketAddr, sync::Arc};

use axum::Router;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

use crate::state::{AppState, SharedState, StreamEvent};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("info".parse().unwrap()),
        )
        .init();

    let sol = solana::SolanaClient::from_env();
    if sol.is_none() {
        tracing::warn!("solana client not configured — settlement broadcast disabled");
    }

    let (tx, _) = broadcast::channel::<StreamEvent>(128);

    let state: SharedState = Arc::new(RwLock::new(AppState::seed(sol, tx)));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .merge(routes::router())
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    tracing::info!("nyxbid-server listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}
