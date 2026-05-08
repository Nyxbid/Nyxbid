mod auction;
mod indexer;
mod routes;
mod solana;
mod state;
mod store;
mod tx;

use std::{net::SocketAddr, sync::Arc};

use axum::Router;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

use crate::indexer::ChainEnvelope;
use crate::state::{AppState, SharedState};
use crate::store::{SharedStore, Store};

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

    // Chain-event broadcast channel feeds: (a) the state-apply task that
    // keeps the store warm, and (b) the SSE/WebSocket subscribers.
    let (chain_tx, _) = broadcast::channel::<ChainEnvelope>(1024);

    // Chain-indexed store. Cold-start backfill runs *before* the indexer
    // starts pushing live events so we don't race RPC with itself.
    let store: SharedStore = Arc::new(RwLock::new(Store::new()));
    if let Some(sol) = sol.as_ref() {
        let mut s = store.write().await;
        if let Err(e) = s.cold_start(sol).await {
            tracing::warn!(error = %e, "cold-start backfill failed; continuing with empty store");
        }
    }

    // Spawn the program-log indexer.
    let indexer_metrics = sol.as_ref().map(|s| {
        let (metrics, _join) = indexer::spawn(s.clone(), chain_tx.clone());
        metrics
    });

    // Spawn the state-apply task: every chain event triggers a re-fetch
    // of the touched account so the store mirrors the chain. This task
    // owns the only writer-side handle to the store.
    if let Some(sol) = sol.clone() {
        let store = Arc::clone(&store);
        let mut rx = chain_tx.subscribe();
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(env) => {
                        let mut s = store.write().await;
                        if let Err(e) = s.apply_event(&env, &sol).await {
                            tracing::warn!(error = %e, signature = %env.signature, "store update failed");
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(skipped = n, "state-apply task lagged behind broadcast channel");
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::info!("state-apply task: chain_tx closed; exiting");
                        return;
                    }
                }
            }
        });
    }

    let state: SharedState = Arc::new(RwLock::new(AppState::new(
        sol,
        store,
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
