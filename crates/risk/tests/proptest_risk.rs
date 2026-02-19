use proptest::prelude::*;
use common::{MarketEvent, EngineState, Position, OrderSide, TradingMode};
use risk::{RiskConfig, RiskManager};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc, RwLock};

proptest! {
    /// Risk rule evaluations on randomized f64 price inputs must never panic.
    #[test]
    fn risk_rules_never_panic_on_extreme_prices(
        entry_price in 0.0001f64..1_000_000.0f64,
        current_price in 0.0001f64..1_000_000.0f64,
        quantity in 0.0001f64..1000.0f64,
    ) {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let config = RiskConfig {
                stop_loss_pct: 0.02,
                take_profit_pct: 0.04,
                max_exposure_per_trade_usd: 10_000.0,
                max_drawdown_pct: 0.15,
            };
            let (_signal_tx, signal_rx) = mpsc::channel(1);
            let (order_tx, _order_rx) = mpsc::channel(1);
            let (risk_event_tx, _risk_event_rx) = mpsc::channel(1);
            let (market_tx, market_rx) = broadcast::channel(8);
            let engine_state = Arc::new(RwLock::new(EngineState::Running));
            let positions = Arc::new(RwLock::new(vec![
                Position {
                    id: "p1".into(),
                    pair: "TESTUSDT".into(),
                    side: OrderSide::Buy,
                    entry_price,
                    quantity,
                    mode: TradingMode::Paper,
                    opened_at: chrono::Utc::now(),
                }
            ]));

            let manager = RiskManager::new(
                config,
                signal_rx,
                order_tx,
                risk_event_tx,
                market_rx,
                engine_state,
                positions,
                10_000.0,
            );

            let handle = tokio::spawn(manager.run());

            // Send one market event — manager should process without panic
            let event = MarketEvent {
                pair: "TESTUSDT".into(),
                price: current_price,
                open: current_price,
                high: current_price,
                low: current_price,
                volume: 1.0,
                is_candle_closed: true,
                timestamp: chrono::Utc::now(),
            };
            let _ = market_tx.send(event);
            tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            handle.abort();
        });
    }
}
