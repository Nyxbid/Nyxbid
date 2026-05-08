mod auction;
mod indexer;
mod intent;
mod routes;
mod solana;
mod state;
mod tx;

use std::{net::SocketAddr, sync::Arc};

use axum::Router;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

use crate::indexer::ChainEvent;
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
        tracing::warn!(
            "solana client not configured \u{2014} on-chain tx prep + indexer disabled \
             (set SOLANA_RPC_URL to enable)"
        );
    }

    // Two broadcast channels: legacy DTO events (going away in commit 7)
    // and chain-decoded events fed by the indexer.
    let (tx, _) = broadcast::channel::<StreamEvent>(128);
    let (chain_tx, _) = broadcast::channel::<ChainEvent>(1024);

    // Spawn the program-log indexer if we have a configured RPC. The
    // task lives for the lifetime of the process and reconnects on
    // disconnect.
    let indexer_metrics = sol.as_ref().map(|s| {
        let (metrics, _join) = indexer::spawn(s.clone(), chain_tx.clone());
        // We deliberately ignore the JoinHandle: the task should outlive
        // any single request and the runtime will cancel it on shutdown.
        metrics
    });

    let state: SharedState = Arc::new(RwLock::new(AppState::seed(
        sol,
        tx,
        chain_tx,
        indexer_metrics,
    )));

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
