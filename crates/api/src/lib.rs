mod auth;
pub mod routes;

use std::sync::Arc;
use std::net::SocketAddr;

use axum::Router;
use sqlx::SqlitePool;
use tokio::sync::{broadcast, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use common::{EngineState, TradingMode};

/// Shared application state injected into every route handler.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub engine_state: Arc<RwLock<EngineState>>,
    pub trading_mode: TradingMode,
    pub dashboard_token: String,
    /// Broadcast channel for streaming log lines to WebSocket clients.
    pub log_tx: broadcast::Sender<String>,
}

/// Build and run the Axum API server.
pub async fn serve(state: AppState, port: u16) {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any);

    let app = Router::new()
        .merge(routes::api_router())
        .merge(routes::ws_router())
        .merge(routes::health_router())
        .merge(routes::static_router())
        .with_state(state)
        .layer(cors);

    info!(%addr, "Dashboard API listening");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
