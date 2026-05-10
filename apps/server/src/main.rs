mod a2a;
mod indexer;
mod routes;
mod solana;
mod state;
mod store;
mod tx;
mod url_privacy;

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
    // Load `.env` files before tracing so the EnvFilter picks up
    // RUST_LOG, and before SolanaClient::from_env so SOLANA_RPC_URL
    // is in scope.
    //
    // Resolution order (first hit wins per key, later files don't
    // overwrite earlier ones — this is the dotenvy semantic):
    //   1. apps/server/.env  (most specific, app-local override)
    //   2. ../../.env        (workspace root, when run from apps/server)
    //   3. ./.env            (workspace root, when run from there)
    //
    // We try every plausible location instead of relying on cwd,
    // because `cargo run -p nyxbid-server` runs from the workspace
    // root while a direct binary invocation runs from elsewhere.
    for path in [
        "apps/server/.env",
        "../../.env",
        ".env",
    ] {
        let _ = dotenvy::from_filename(path);
    }

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive("info".parse().unwrap()),
        )
        .init();

    let sol = solana::SolanaClient::from_env();
    if sol.is_none() {
        tracing::warn!(
            "solana client not configured \u{2014} on-chain tx prep + indexer disabled \
             (set SOLANA_RPC_URL in .env or the process environment)"
        );
    } else {
        let rpc = sol.as_ref().unwrap();
        tracing::info!(
            rpc_url = %url_privacy::public_origin(&rpc.rpc_url),
            "solana client configured"
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
    // of the touched account(s) and a short write-lock to apply the
    // fetched data. The two-phase split (fetch_updates → apply_updates)
    // keeps the write lock held only for sub-microsecond in-memory
    // mutations; the (potentially slow) RPC calls happen with no lock.
    if let Some(sol) = sol.clone() {
        let store = Arc::clone(&store);
        let mut rx = chain_tx.subscribe();
        tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(env) => {
                        // Phase 1: fetch over RPC — no lock held.
                        let updates = match store::fetch_updates(&env, &sol).await {
                            Ok(u) => u,
                            Err(e) => {
                                tracing::warn!(
                                    error = %e,
                                    signature = %env.signature,
                                    "store fetch failed; skipping event"
                                );
                                continue;
                            }
                        };
                        // Phase 2: apply — short write lock.
                        store.write().await.apply_updates(updates);
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
