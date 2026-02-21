use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::{broadcast, mpsc, RwLock};
use tracing::info;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::EnvFilter;

use common::{Config, TradingMode};
use engine::{BinanceClient, Engine, OrderExecutor};
use paper::PaperClient;
use risk::{RiskConfig, RiskManager};
use strategy::{StrategyFileConfig, StrategyRegistry};
use telegram_ctrl::{start_bot, BotDeps};

/// A tracing layer that forwards formatted log lines to a broadcast channel
/// so the dashboard WebSocket can stream them in real time.
struct BroadcastLayer {
    tx: broadcast::Sender<String>,
}

impl<S: tracing::Subscriber> tracing_subscriber::Layer<S> for BroadcastLayer {
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut visitor = MessageVisitor(String::new());
        event.record(&mut visitor);
        let level = event.metadata().level();
        let target = event.metadata().target();
        let line = format!("{level} {target}: {}", visitor.0);
        let _ = self.tx.send(line);
    }
}

struct MessageVisitor(String);

impl tracing::field::Visit for MessageVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            self.0 = format!("{value:?}");
        } else if !self.0.is_empty() {
            self.0.push_str(&format!(" {}={:?}", field.name(), value));
        } else {
            self.0 = format!("{}={:?}", field.name(), value);
        }
    }
}

#[tokio::main]
async fn main() {
    // ── Shared log broadcast (created early so tracing layer can use it) ────
    let (log_tx, _) = broadcast::channel::<String>(1024);

    // ── Logging ──────────────────────────────────────────────────────────────
    let broadcast_layer = BroadcastLayer { tx: log_tx.clone() };
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env().add_directive("info".parse().unwrap()))
        .with(tracing_subscriber::fmt::layer())
        .with(broadcast_layer)
        .init();

    // ── Config ────────────────────────────────────────────────────────────────
    let cfg = Config::from_env();
    info!(mode = %cfg.trading_mode, "ClawBot starting");

    // ── Database ──────────────────────────────────────────────────────────────
    let db = SqlitePool::connect(&cfg.database_url)
        .await
        .unwrap_or_else(|e| panic!("Failed to connect to database: {e}"));
    sqlx::migrate!("../../migrations")
        .run(&db)
        .await
        .unwrap_or_else(|e| panic!("Database migration failed: {e}"));
    info!("Database ready");

    // ── Shared state ──────────────────────────────────────────────────────────
    let open_positions: Arc<RwLock<Vec<common::Position>>> = Arc::new(RwLock::new(Vec::new()));

    // ── Engine ────────────────────────────────────────────────────────────────
    // Pairs to stream — read from strategy config
    let strategy_file = StrategyFileConfig::load(&cfg.strategy_config_path);
    let pairs: Vec<String> = {
        let mut seen = std::collections::HashSet::new();
        strategy_file
            .strategies
            .iter()
            .filter_map(|s| {
                if seen.insert(s.pair.clone()) {
                    Some(s.pair.clone())
                } else {
                    None
                }
            })
            .collect()
    };

    let (engine, engine_handle) = Engine::new(pairs);
    // Use the engine's own state — single source of truth
    let engine_state = engine_handle.state_handle();

    // ── Exchange client (injected based on TRADING_MODE) ──────────────────────
    let exchange_client: Arc<dyn common::ExchangeClient> = match cfg.trading_mode {
        TradingMode::Live => {
            info!("Live trading mode — using BinanceClient");
            Arc::new(BinanceClient::new(
                &cfg.binance_api_key,
                &cfg.binance_secret,
            ))
        }
        TradingMode::Paper => {
            info!(
                slippage_bps = cfg.paper_slippage_bps,
                "Paper trading mode — using PaperClient"
            );
            Arc::new(PaperClient::new(
                cfg.paper_initial_balance,
                cfg.paper_slippage_bps,
            ))
        }
    };

    // ── Channels ──────────────────────────────────────────────────────────────
    let (signal_tx, signal_rx) = mpsc::channel::<common::Signal>(128);
    let (order_tx, order_rx) = mpsc::channel::<common::Order>(128);
    let (risk_event_tx, mut risk_event_rx) = mpsc::channel::<common::RiskEvent>(64);
    let market_rx_strategy = engine_handle.subscribe_market();
    let market_rx_risk = engine_handle.subscribe_market();

    // ── Strategy registry ─────────────────────────────────────────────────────
    let registry = StrategyRegistry::from_config(&strategy_file);

    // ── Risk manager ──────────────────────────────────────────────────────────
    let risk_cfg = RiskConfig::default(); // TODO: load from file
    let risk_manager = RiskManager::new(
        risk_cfg,
        signal_rx,
        order_tx,
        risk_event_tx.clone(),
        market_rx_risk,
        engine_state.clone(),
        open_positions.clone(),
        cfg.paper_initial_balance,
    );

    // ── Order executor ────────────────────────────────────────────────────────
    let executor = OrderExecutor::new(
        order_rx,
        risk_event_tx.clone(),
        exchange_client,
        db.clone(),
        cfg.trading_mode,
    );

    // ── Telegram C2 ───────────────────────────────────────────────────────────
    let allowed_ids: Vec<i64> = cfg.telegram_allowed_user_ids.clone();
    let bot_deps = BotDeps {
        command_tx: {
            // Create a command channel bridged to the engine handle
            let (tx, mut rx) = mpsc::channel::<common::EngineCommand>(32);
            let handle = engine_handle.clone();
            tokio::spawn(async move {
                while let Some(cmd) = rx.recv().await {
                    handle.send(cmd).await;
                }
            });
            tx
        },
        engine_state: engine_state.clone(),
        trading_mode: cfg.trading_mode,
        allowed_user_ids: Arc::new(allowed_ids),
        alert_rx: Arc::new(tokio::sync::Mutex::new({
            let (_, rx) = mpsc::channel(1);
            rx
        })),
    };

    // ── Log buffer (keeps recent logs for new dashboard clients) ─────────────
    let log_buffer = api::LogBuffer::new(500);
    {
        let buffer = log_buffer.clone();
        let mut rx = log_tx.subscribe();
        tokio::spawn(async move {
            while let Ok(line) = rx.recv().await {
                buffer.push(line).await;
            }
        });
    }

    // ── Dashboard API ─────────────────────────────────────────────────────────
    let api_state = api::AppState {
        db: db.clone(),
        engine_state: engine_state.clone(),
        trading_mode: cfg.trading_mode,
        dashboard_token: cfg.dashboard_token.clone(),
        initial_balance: cfg.paper_initial_balance,
        log_tx: log_tx.clone(),
        log_buffer,
    };

    // ── Risk event forwarder (sends alerts to Telegram) ───────────────────────
    let telegram_token = cfg.telegram_token.clone();
    let alert_user_ids: Vec<i64> = cfg.telegram_allowed_user_ids.clone();
    tokio::spawn(async move {
        let bot = teloxide::Bot::new(telegram_token);
        let chat_ids: Vec<teloxide::types::ChatId> = alert_user_ids
            .iter()
            .map(|&id| teloxide::types::ChatId(id))
            .collect();

        while let Some(event) = risk_event_rx.recv().await {
            let msg = match event {
                common::RiskEvent::StopLossTriggered { pair, close_price } => {
                    format!(
                        "⚠️ Stop-loss triggered on {pair}. Position closed at {close_price:.4}."
                    )
                }
                common::RiskEvent::TakeProfitTriggered { pair, close_price } => {
                    format!(
                        "✅ Take-profit triggered on {pair}. Position closed at {close_price:.4}."
                    )
                }
                common::RiskEvent::OrderFailed { pair, error } => {
                    format!("🚨 Order failed on {pair}: {error}")
                }
                common::RiskEvent::DrawdownHaltEntered { drawdown_pct } => {
                    format!("🛑 Max drawdown breached ({:.1}%). Engine halted. Use /reset-drawdown to resume.", drawdown_pct * 100.0)
                }
                common::RiskEvent::DrawdownHaltExited => {
                    "✅ Drawdown halt cleared. Engine resuming.".to_string()
                }
                common::RiskEvent::OrderRejected { signal, reason } => {
                    format!("⛔ Order rejected on {}: {reason}", signal.pair())
                }
            };
            telegram_ctrl::commands::send_alert(&bot, &chat_ids, &msg).await;
        }
    });

    // ── Spawn all tasks ───────────────────────────────────────────────────────
    let port = cfg.dashboard_port;
    tokio::spawn(engine.run());
    tokio::spawn(registry.run(market_rx_strategy, signal_tx, engine_state.clone()));
    tokio::spawn(risk_manager.run());
    tokio::spawn(executor.run());
    tokio::spawn(start_bot(cfg.telegram_token.clone(), bot_deps));
    tokio::spawn(api::serve(api_state, port));

    // Keep main alive
    info!("All subsystems started. Waiting for shutdown signal.");
    tokio::signal::ctrl_c().await.unwrap();
    info!("Shutdown signal received. Exiting.");
}
