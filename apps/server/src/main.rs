mod mock;
mod routes;
pub mod solana;
mod x402;

use std::{net::SocketAddr, sync::Arc};

use axum::Router;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

pub struct AppState {
    pub agents: Vec<payq_types::Agent>,
    pub receipts: Vec<payq_types::SpendReceipt>,
    pub policies: Vec<payq_types::Policy>,
    pub solana: Option<solana::SolanaClient>,
    pub tx: broadcast::Sender<payq_types::SpendReceipt>,
}

pub type SharedState = Arc<RwLock<AppState>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .init();

    let sol = solana::SolanaClient::from_env();
    if sol.is_none() {
        tracing::warn!("solana client not configured — on-chain recording disabled");
    }

    let (tx, _) = broadcast::channel::<payq_types::SpendReceipt>(64);

    let state: SharedState = Arc::new(RwLock::new(AppState {
        solana: sol,
        tx,
        ..mock::seed()
    }));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .merge(routes::router())
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 3001));
    tracing::info!("payq-server listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await.expect("bind");
    axum::serve(listener, app).await.expect("serve");
}
