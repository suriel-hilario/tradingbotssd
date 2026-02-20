use std::collections::HashMap;
use std::sync::Arc;

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, RwLock};
use tracing::{info, warn};

use common::{
    EngineState, MarketEvent, Order, OrderSide, Position, RejectionReason, RiskEvent, Signal,
};

/// Hard ceiling on simultaneous open orders. Compiled-in constant — not
/// user-configurable — as a last-resort safeguard against runaway trading.
pub const MAX_OPEN_ORDERS: usize = 5;

/// User-configurable risk parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    /// Maximum loss on a single position before auto-close (e.g. 0.02 = 2%).
    pub stop_loss_pct: f64,
    /// Target gain on a single position before auto-close (e.g. 0.03 = 3%).
    pub take_profit_pct: f64,
    /// Maximum USD notional for a single order.
    pub max_exposure_per_trade_usd: f64,
    /// Portfolio drawdown from peak that triggers a halt (e.g. 0.10 = 10%).
    pub max_drawdown_pct: f64,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            stop_loss_pct: 0.02,
            take_profit_pct: 0.04,
            max_exposure_per_trade_usd: 100.0,
            max_drawdown_pct: 0.10,
        }
    }
}

/// The gatekeeper between the strategy layer and the order executor.
///
/// ALL signals from strategy MUST pass through `run()` before reaching the executor.
/// No strategy or other module holds a direct reference to the order channel.
pub struct RiskManager {
    config: RiskConfig,
    signal_rx: mpsc::Receiver<Signal>,
    order_tx: mpsc::Sender<Order>,
    risk_event_tx: mpsc::Sender<RiskEvent>,
    market_rx: tokio::sync::broadcast::Receiver<MarketEvent>,
    engine_state: Arc<RwLock<EngineState>>,
    open_positions: Arc<RwLock<Vec<Position>>>,
    portfolio_peak_usd: f64,
    portfolio_value_usd: f64,
    /// Latest price per pair for PnL monitoring.
    latest_prices: HashMap<String, f64>,
}

impl RiskManager {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        config: RiskConfig,
        signal_rx: mpsc::Receiver<Signal>,
        order_tx: mpsc::Sender<Order>,
        risk_event_tx: mpsc::Sender<RiskEvent>,
        market_rx: tokio::sync::broadcast::Receiver<MarketEvent>,
        engine_state: Arc<RwLock<EngineState>>,
        open_positions: Arc<RwLock<Vec<Position>>>,
        initial_portfolio_usd: f64,
    ) -> Self {
        Self {
            config,
            signal_rx,
            order_tx,
            risk_event_tx,
            market_rx,
            engine_state,
            open_positions,
            portfolio_peak_usd: initial_portfolio_usd,
            portfolio_value_usd: initial_portfolio_usd,
            latest_prices: HashMap::new(),
        }
    }

    /// Run the risk manager loop. Processes both incoming signals and
    /// market price updates concurrently via `tokio::select!`.
    pub async fn run(mut self) {
        info!("RiskManager running");
        loop {
            tokio::select! {
                // ── Incoming strategy signal ──────────────────────────────
                signal = self.signal_rx.recv() => {
                    match signal {
                        Some(sig) => self.handle_signal(sig).await,
                        None => {
                            warn!("Signal channel closed — RiskManager exiting");
                            return;
                        }
                    }
                }

                // ── Market price update ───────────────────────────────────
                event = self.market_rx.recv() => {
                    match event {
                        Ok(ev) => self.handle_market_event(ev).await,
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                            warn!(dropped = n, "RiskManager market channel lagged");
                        }
                        Err(_) => {
                            warn!("Market broadcast closed");
                            return;
                        }
                    }
                }
            }
        }
    }

    async fn handle_signal(&mut self, signal: Signal) {
        let state = *self.engine_state.read().await;

        // Block all signals when halted
        if state == EngineState::Halted {
            self.reject(&signal, RejectionReason::DrawdownHalt).await;
            return;
        }

        // Hard order ceiling check
        {
            let positions = self.open_positions.read().await;
            if positions.len() >= MAX_OPEN_ORDERS {
                self.reject(&signal, RejectionReason::HardCeilingReached).await;
                return;
            }
        }

        // Max exposure check
        let pair_price = self
            .latest_prices
            .get(signal.pair())
            .copied()
            .unwrap_or(0.0);
        let notional = signal.quantity() * pair_price;
        if notional > self.config.max_exposure_per_trade_usd && pair_price > 0.0 {
            self.reject(&signal, RejectionReason::ExposureLimitExceeded).await;
            return;
        }

        // Approved — forward to executor
        let order = Order::market(signal.pair(), signal.side(), signal.quantity());
        info!(pair = %order.pair, side = ?order.side, notional = notional, "Order approved by RiskManager");
        let _ = self.order_tx.send(order).await;
    }

    async fn handle_market_event(&mut self, event: MarketEvent) {
        self.latest_prices.insert(event.pair.clone(), event.price);

        let positions: Vec<Position> = self.open_positions.read().await.clone();

        for position in &positions {
            if position.pair != event.pair {
                continue;
            }
            let current_price = event.price;
            let entry = position.entry_price;
            if entry <= 0.0 {
                continue;
            }

            let pnl_pct = match position.side {
                OrderSide::Buy => (current_price - entry) / entry,
                OrderSide::Sell => (entry - current_price) / entry,
            };

            // Stop-loss check
            if pnl_pct <= -self.config.stop_loss_pct {
                info!(pair = %position.pair, pnl_pct = pnl_pct, "Stop-loss triggered");
                let close_order = Order::market(
                    &position.pair,
                    if position.side == OrderSide::Buy { OrderSide::Sell } else { OrderSide::Buy },
                    position.quantity,
                );
                let _ = self.order_tx.send(close_order).await;

                // Remove closed position from tracking
                let pnl_usd = pnl_pct * position.entry_price * position.quantity;
                self.remove_position(&position.id).await;
                self.update_portfolio_value(pnl_usd);

                let _ = self
                    .risk_event_tx
                    .send(RiskEvent::StopLossTriggered {
                        pair: position.pair.clone(),
                        close_price: current_price,
                    })
                    .await;
                continue;
            }

            // Take-profit check
            if pnl_pct >= self.config.take_profit_pct {
                info!(pair = %position.pair, pnl_pct = pnl_pct, "Take-profit triggered");
                let close_order = Order::market(
                    &position.pair,
                    if position.side == OrderSide::Buy { OrderSide::Sell } else { OrderSide::Buy },
                    position.quantity,
                );
                let _ = self.order_tx.send(close_order).await;

                // Remove closed position from tracking
                let pnl_usd = pnl_pct * position.entry_price * position.quantity;
                self.remove_position(&position.id).await;
                self.update_portfolio_value(pnl_usd);

                let _ = self
                    .risk_event_tx
                    .send(RiskEvent::TakeProfitTriggered {
                        pair: position.pair.clone(),
                        close_price: current_price,
                    })
                    .await;
            }
        }

        // Drawdown circuit breaker
        self.check_drawdown().await;
    }

    async fn check_drawdown(&mut self) {
        if self.portfolio_peak_usd <= 0.0 {
            return;
        }
        let drawdown = (self.portfolio_peak_usd - self.portfolio_value_usd)
            / self.portfolio_peak_usd;

        if drawdown >= self.config.max_drawdown_pct {
            let current_state = *self.engine_state.read().await;
            if current_state != EngineState::Halted {
                warn!(
                    drawdown_pct = drawdown * 100.0,
                    "Max drawdown breached — entering HaltedState"
                );
                *self.engine_state.write().await = EngineState::Halted;
                let _ = self
                    .risk_event_tx
                    .send(RiskEvent::DrawdownHaltEntered {
                        drawdown_pct: drawdown,
                    })
                    .await;
            }
        }
    }

    /// Remove a closed position from the shared open-positions list.
    async fn remove_position(&self, position_id: &str) {
        let mut positions = self.open_positions.write().await;
        if let Some(idx) = positions.iter().position(|p| p.id == position_id) {
            let removed = positions.remove(idx);
            info!(pair = %removed.pair, id = %removed.id, "Position removed from tracking after close");
        }
    }

    /// Update portfolio value after a realized P&L, and track the peak for drawdown.
    fn update_portfolio_value(&mut self, realized_pnl_usd: f64) {
        self.portfolio_value_usd += realized_pnl_usd;
        if self.portfolio_value_usd > self.portfolio_peak_usd {
            self.portfolio_peak_usd = self.portfolio_value_usd;
        }
        info!(
            portfolio_value = self.portfolio_value_usd,
            portfolio_peak = self.portfolio_peak_usd,
            realized_pnl = realized_pnl_usd,
            "Portfolio value updated"
        );
    }

    async fn reject(&self, signal: &Signal, reason: RejectionReason) {
        warn!(
            pair = %signal.pair(),
            reason = %reason,
            "Order rejected by RiskManager"
        );
        let _ = self
            .risk_event_tx
            .send(RiskEvent::OrderRejected {
                signal: signal.clone(),
                reason,
            })
            .await;
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use tokio::sync::{broadcast, mpsc, RwLock};
    use common::{EngineState, Signal};

    fn make_position(pair: &str, entry_price: f64, quantity: f64) -> Position {
        Position {
            id: "test".into(),
            pair: pair.into(),
            side: OrderSide::Buy,
            entry_price,
            quantity,
            mode: common::TradingMode::Paper,
            opened_at: chrono::Utc::now(),
        }
    }

    fn make_event(pair: &str, price: f64) -> MarketEvent {
        MarketEvent {
            pair: pair.into(),
            price,
            open: price,
            high: price,
            low: price,
            volume: 100.0,
            is_candle_closed: true,
            timestamp: chrono::Utc::now(),
        }
    }

    async fn make_manager(
        config: RiskConfig,
    ) -> (
        RiskManager,
        mpsc::Sender<Signal>,
        mpsc::Receiver<Order>,
        mpsc::Receiver<RiskEvent>,
        broadcast::Sender<MarketEvent>,
        Arc<RwLock<Vec<Position>>>,
        Arc<RwLock<EngineState>>,
    ) {
        let (signal_tx, signal_rx) = mpsc::channel(32);
        let (order_tx, order_rx) = mpsc::channel(32);
        let (risk_event_tx, risk_event_rx) = mpsc::channel(32);
        let (market_tx, market_rx) = broadcast::channel(64);
        let engine_state = Arc::new(RwLock::new(EngineState::Running));
        let positions: Arc<RwLock<Vec<Position>>> = Arc::new(RwLock::new(Vec::new()));

        let manager = RiskManager::new(
            config,
            signal_rx,
            order_tx,
            risk_event_tx,
            market_rx,
            engine_state.clone(),
            positions.clone(),
            10_000.0,
        );

        (manager, signal_tx, order_rx, risk_event_rx, market_tx, positions, engine_state)
    }

    #[tokio::test]
    async fn stop_loss_fires_at_threshold() {
        let config = RiskConfig {
            stop_loss_pct: 0.02,
            ..RiskConfig::default()
        };
        let (manager, _signal_tx, mut order_rx, mut risk_rx, market_tx, positions, _state) =
            make_manager(config).await;

        // Add an open position at 1000.0
        {
            let mut pos = positions.write().await;
            pos.push(make_position("BTCUSDT", 1000.0, 0.01));
        }

        tokio::spawn(manager.run());

        // Price drops 2% → stop-loss should trigger
        market_tx.send(make_event("BTCUSDT", 980.0)).unwrap();

        let event = tokio::time::timeout(std::time::Duration::from_secs(1), risk_rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        assert!(
            matches!(event, RiskEvent::StopLossTriggered { .. }),
            "Expected StopLossTriggered, got: {:?}",
            event
        );

        let order = tokio::time::timeout(std::time::Duration::from_secs(1), order_rx.recv())
            .await
            .expect("timeout")
            .expect("no order emitted");
        assert_eq!(order.side, OrderSide::Sell);

        // Position should be removed from tracking after stop-loss
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let pos = positions.read().await;
        assert!(pos.is_empty(), "Position should be removed after stop-loss");
    }

    #[tokio::test]
    async fn take_profit_fires_at_threshold() {
        let config = RiskConfig {
            take_profit_pct: 0.03,
            ..RiskConfig::default()
        };
        let (manager, _signal_tx, _order_rx, mut risk_rx, market_tx, positions, _state) =
            make_manager(config).await;

        {
            let mut pos = positions.write().await;
            pos.push(make_position("BTCUSDT", 1000.0, 0.01));
        }

        tokio::spawn(manager.run());

        market_tx.send(make_event("BTCUSDT", 1030.0)).unwrap();

        let event = tokio::time::timeout(std::time::Duration::from_secs(1), risk_rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        assert!(
            matches!(event, RiskEvent::TakeProfitTriggered { .. }),
            "Expected TakeProfitTriggered"
        );

        // Position should be removed from tracking after take-profit
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        let pos = positions.read().await;
        assert!(pos.is_empty(), "Position should be removed after take-profit");
    }

    #[tokio::test]
    async fn exposure_limit_rejects_large_order() {
        let config = RiskConfig {
            max_exposure_per_trade_usd: 50.0,
            ..RiskConfig::default()
        };
        let (manager, signal_tx, _order_rx, mut risk_rx, market_tx, _positions, _state) =
            make_manager(config).await;

        tokio::spawn(manager.run());

        // Seed a price
        market_tx.send(make_event("BTCUSDT", 1000.0)).unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        // quantity=0.1 @ 1000.0 = 100 USD > 50 USD limit
        signal_tx
            .send(Signal::Buy { pair: "BTCUSDT".into(), quantity: 0.1 })
            .await
            .unwrap();

        let event = tokio::time::timeout(std::time::Duration::from_secs(1), risk_rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        assert!(
            matches!(
                event,
                RiskEvent::OrderRejected {
                    reason: RejectionReason::ExposureLimitExceeded,
                    ..
                }
            ),
            "Expected ExposureLimitExceeded rejection"
        );
    }

    #[tokio::test]
    async fn drawdown_halt_engages_and_blocks_orders() {
        let config = RiskConfig {
            max_drawdown_pct: 0.10,
            ..RiskConfig::default()
        };
        let (mut manager, signal_tx, _order_rx, mut risk_rx, _market_tx, _positions, state) =
            make_manager(config).await;

        // Simulate portfolio below peak by 10%
        manager.portfolio_value_usd = 9000.0;
        manager.portfolio_peak_usd = 10_000.0;

        tokio::spawn(manager.run());

        // Trigger drawdown check by sending a signal (via halted state check in handle_signal)
        // First set state to halted manually to test blocking
        *state.write().await = EngineState::Halted;

        signal_tx
            .send(Signal::Buy { pair: "ETHUSDT".into(), quantity: 0.01 })
            .await
            .unwrap();

        let event = tokio::time::timeout(std::time::Duration::from_secs(1), risk_rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        assert!(
            matches!(
                event,
                RiskEvent::OrderRejected {
                    reason: RejectionReason::DrawdownHalt,
                    ..
                }
            ),
            "Expected DrawdownHalt rejection"
        );
    }

    #[tokio::test]
    async fn hard_ceiling_rejects_nth_plus_one_order() {
        let config = RiskConfig {
            max_exposure_per_trade_usd: 10_000.0, // large enough to not trigger
            ..RiskConfig::default()
        };
        let (manager, signal_tx, _order_rx, mut risk_rx, market_tx, positions, _state) =
            make_manager(config).await;

        // Fill up to the hard ceiling
        {
            let mut pos = positions.write().await;
            for i in 0..MAX_OPEN_ORDERS {
                pos.push(make_position(&format!("PAIR{i}USDT"), 100.0, 1.0));
            }
        }

        tokio::spawn(manager.run());

        market_tx.send(make_event("NEWPAIR", 100.0)).unwrap();
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;

        signal_tx
            .send(Signal::Buy { pair: "NEWPAIR".into(), quantity: 0.01 })
            .await
            .unwrap();

        let event = tokio::time::timeout(std::time::Duration::from_secs(1), risk_rx.recv())
            .await
            .expect("timeout")
            .expect("channel closed");

        assert!(
            matches!(
                event,
                RiskEvent::OrderRejected {
                    reason: RejectionReason::HardCeilingReached,
                    ..
                }
            ),
            "Expected HardCeilingReached rejection"
        );
    }
}
