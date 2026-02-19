use axum::{extract::State, routing::get, Json, Router};
use serde_json::{json, Value};

use crate::AppState;

pub fn health_router() -> Router<AppState> {
    Router::new().route("/healthz", get(healthz))
}

/// Health check endpoint — no auth required.
/// Used by systemd post-deploy check and ops scripts.
async fn healthz(State(state): State<AppState>) -> Json<Value> {
    let engine_state = *state.engine_state.read().await;
    Json(json!({
        "status": "ok",
        "engine": engine_state.to_string(),
        "mode": state.trading_mode.to_string(),
    }))
}
