use axum::{
    extract::{Query, State},
    http::StatusCode,
    middleware,
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tracing::warn;

use crate::{auth::require_auth, AppState};

pub fn api_router() -> Router<AppState> {
    Router::new()
        .route("/api/portfolio", get(get_portfolio))
        .route("/api/trades", get(get_trades))
        .route("/api/performance", get(get_performance))
        .route("/api/config", get(get_config).post(post_config))
        .route_layer(middleware::from_fn_with_state(
            // Placeholder AppState for the middleware layer factory.
            // Axum replaces this with the real state at runtime via with_state().
            AppState {
                db: sqlx::SqlitePool::connect_lazy("sqlite::memory:").unwrap(),
                engine_state: Default::default(),
                trading_mode: common::TradingMode::Paper,
                dashboard_token: String::new(),
                log_tx: tokio::sync::broadcast::channel::<String>(1).0,
            },
            require_auth,
        ))
}

// ─── Portfolio ────────────────────────────────────────────────────────────────

async fn get_portfolio(State(state): State<AppState>) -> Json<Value> {
    let positions = sqlx::query!(
        r#"SELECT id, pair, side, entry_price, quantity, mode, opened_at FROM positions"#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let pos_json: Vec<Value> = positions
        .iter()
        .map(|p| {
            json!({
                "id": p.id,
                "pair": p.pair,
                "side": p.side,
                "entry_price": p.entry_price,
                "quantity": p.quantity,
                "mode": p.mode,
                "opened_at": p.opened_at,
            })
        })
        .collect();

    Json(json!({
        "positions": pos_json,
        "total_open": pos_json.len(),
    }))
}

// ─── Trades ───────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct TradesQuery {
    page: Option<i64>,
    limit: Option<i64>,
    pair: Option<String>,
}

async fn get_trades(
    State(state): State<AppState>,
    Query(q): Query<TradesQuery>,
) -> Json<Value> {
    let page = q.page.unwrap_or(1).max(1);
    let limit = q.limit.unwrap_or(50).min(200);
    let offset = (page - 1) * limit;

    if let Some(pair) = &q.pair {
        let rows = sqlx::query!(
            r#"SELECT id, pair, side, entry_price, exit_price, quantity, pnl_usd, mode, opened_at, closed_at
               FROM trades WHERE pair = ?1 ORDER BY closed_at DESC LIMIT ?2 OFFSET ?3"#,
            pair, limit, offset
        )
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        let total: i32 = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM trades WHERE pair = ?1", pair
        )
        .fetch_one(&state.db)
        .await
        .unwrap_or(0);

        let trades: Vec<Value> = rows.iter().map(|t| json!({
            "id": t.id, "pair": t.pair, "side": t.side,
            "entry_price": t.entry_price, "exit_price": t.exit_price,
            "quantity": t.quantity, "pnl_usd": t.pnl_usd,
            "mode": t.mode, "opened_at": t.opened_at, "closed_at": t.closed_at,
        })).collect();
        Json(json!({ "trades": trades, "total": total, "page": page, "limit": limit }))
    } else {
        let rows = sqlx::query!(
            r#"SELECT id, pair, side, entry_price, exit_price, quantity, pnl_usd, mode, opened_at, closed_at
               FROM trades ORDER BY closed_at DESC LIMIT ?1 OFFSET ?2"#,
            limit, offset
        )
        .fetch_all(&state.db)
        .await
        .unwrap_or_default();

        let total: i32 = sqlx::query_scalar!("SELECT COUNT(*) FROM trades")
            .fetch_one(&state.db)
            .await
            .unwrap_or(0);

        let trades: Vec<Value> = rows.iter().map(|t| json!({
            "id": t.id, "pair": t.pair, "side": t.side,
            "entry_price": t.entry_price, "exit_price": t.exit_price,
            "quantity": t.quantity, "pnl_usd": t.pnl_usd,
            "mode": t.mode, "opened_at": t.opened_at, "closed_at": t.closed_at,
        })).collect();
        Json(json!({ "trades": trades, "total": total, "page": page, "limit": limit }))
    }
}

// ─── Performance ──────────────────────────────────────────────────────────────

async fn get_performance(State(state): State<AppState>) -> Json<Value> {
    let trades = sqlx::query!(
        r#"SELECT pnl_usd, closed_at FROM trades ORDER BY closed_at ASC"#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    if trades.is_empty() {
        return Json(json!({
            "equity_curve": [],
            "win_rate": 0.0,
            "total_pnl_usd": 0.0,
            "trade_count": 0,
            "max_drawdown_pct": 0.0,
        }));
    }

    let mut equity = 10_000.0f64;
    let mut peak = equity;
    let mut max_dd = 0.0f64;
    let mut wins = 0usize;
    let mut curve: Vec<Value> = Vec::new();

    for t in &trades {
        equity += t.pnl_usd;
        if equity > peak { peak = equity; }
        let dd = (peak - equity) / peak;
        if dd > max_dd { max_dd = dd; }
        if t.pnl_usd > 0.0 { wins += 1; }
        curve.push(json!({ "timestamp": t.closed_at, "value": equity }));
    }

    let win_rate = wins as f64 / trades.len() as f64;
    let total_pnl: f64 = trades.iter().map(|t| t.pnl_usd).sum();

    Json(json!({
        "equity_curve": curve,
        "win_rate": win_rate,
        "total_pnl_usd": total_pnl,
        "trade_count": trades.len(),
        "max_drawdown_pct": max_dd,
    }))
}

// ─── Config ───────────────────────────────────────────────────────────────────

async fn get_config() -> Json<Value> {
    Json(json!({ "message": "Config endpoint active." }))
}

async fn post_config(Json(_body): Json<Value>) -> (StatusCode, Json<Value>) {
    warn!("POST /api/config received");
    (StatusCode::OK, Json(json!({ "status": "accepted" })))
}
