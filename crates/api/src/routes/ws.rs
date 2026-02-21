use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::Response,
    routing::get,
    Router,
};
use serde::Deserialize;
use tracing::warn;

use crate::AppState;

pub fn ws_router() -> Router<AppState> {
    Router::new().route("/ws/logs", get(ws_logs_handler))
}

#[derive(Deserialize)]
struct WsQuery {
    token: Option<String>,
}

/// WebSocket endpoint that streams real-time log lines to the dashboard.
/// Auth via query param `?token=<DASHBOARD_TOKEN>` (header auth not supported
/// in browser WebSocket API).
async fn ws_logs_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(q): Query<WsQuery>,
) -> Response {
    // Authenticate via query token (browsers can't set custom WS headers)
    let authed = q
        .token
        .as_deref()
        .map(|t| t == state.dashboard_token)
        .unwrap_or(false);

    if !authed {
        return axum::response::IntoResponse::into_response((
            axum::http::StatusCode::UNAUTHORIZED,
            "unauthorized",
        ));
    }

    let log_buffer = state.log_buffer.clone();
    let log_rx = state.log_tx.subscribe();
    ws.on_upgrade(move |socket| handle_ws(socket, log_rx, log_buffer))
}

async fn handle_ws(
    mut socket: WebSocket,
    mut log_rx: tokio::sync::broadcast::Receiver<String>,
    log_buffer: crate::LogBuffer,
) {
    // Send log history first so the client sees previous logs
    let history = log_buffer.snapshot().await;
    for line in history {
        if socket.send(Message::Text(line)).await.is_err() {
            return;
        }
    }

    // Then stream live logs
    loop {
        match log_rx.recv().await {
            Ok(line) => {
                if socket.send(Message::Text(line)).await.is_err() {
                    break;
                }
            }
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                warn!(dropped = n, "WebSocket log client lagged");
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                break;
            }
        }
    }
}
