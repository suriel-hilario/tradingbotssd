mod auth;
pub mod routes;

use std::collections::VecDeque;
use std::sync::Arc;
use std::net::SocketAddr;

use axum::Router;
use sqlx::SqlitePool;
use tokio::sync::{broadcast, Mutex, RwLock};
use tower_http::cors::{Any, CorsLayer};
use tracing::info;

use common::{EngineState, TradingMode};

/// Ring buffer that keeps recent log lines so new clients get history.
#[derive(Clone)]
pub struct LogBuffer {
    inner: Arc<Mutex<VecDeque<String>>>,
    capacity: usize,
}

impl LogBuffer {
    pub fn new(capacity: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(VecDeque::with_capacity(capacity))),
            capacity,
        }
    }

    pub async fn push(&self, line: String) {
        let mut buf = self.inner.lock().await;
        if buf.len() >= self.capacity {
            buf.pop_front();
        }
        buf.push_back(line);
    }

    pub async fn snapshot(&self) -> Vec<String> {
        self.inner.lock().await.iter().cloned().collect()
    }
}

/// Shared application state injected into every route handler.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    pub engine_state: Arc<RwLock<EngineState>>,
    pub trading_mode: TradingMode,
    pub dashboard_token: String,
    pub initial_balance: f64,
    /// Broadcast channel for streaming log lines to WebSocket clients.
    pub log_tx: broadcast::Sender<String>,
    /// Recent log history for new clients.
    pub log_buffer: LogBuffer,
}

/// Build and run the Axum API server.
pub async fn serve(state: AppState, port: u16) {
    let addr = SocketAddr::from(([0, 0, 0, 0], port));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_headers(Any)
        .allow_methods(Any);

    let app = Router::new()
        .merge(routes::api_router(state.clone()))
        .merge(routes::ws_router())
        .merge(routes::health_router())
        .merge(routes::static_router())
        .with_state(state)
        .layer(cors);

    info!(%addr, "Dashboard API listening");
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
