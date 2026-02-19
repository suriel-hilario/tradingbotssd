use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::broadcast;
use tokio::sync::mpsc;
use tracing::{info, warn};

use common::{EngineState, MarketEvent, Signal};

use crate::config::{StrategyConfig, StrategyFileConfig};
use crate::indicators::{MacdIndicator, RsiIndicator};
use crate::Strategy;

/// Holds all active strategy instances and dispatches market events to them.
pub struct StrategyRegistry {
    strategies: Vec<Box<dyn Strategy>>,
    /// Per-pair rolling window of recent closed candles for indicator calculation.
    price_history: HashMap<String, Vec<f64>>,
    max_history: usize,
}

impl StrategyRegistry {
    const DEFAULT_MAX_HISTORY: usize = 200;

    /// Build the registry from config, exiting on unknown strategy types.
    pub fn from_config(file_cfg: &StrategyFileConfig) -> Self {
        let mut strategies: Vec<Box<dyn Strategy>> = Vec::new();

        for cfg in &file_cfg.strategies {
            let strategy = build_strategy(cfg).unwrap_or_else(|e| {
                panic!("Unknown strategy type '{}': {e}", cfg.strategy_type)
            });
            info!(name = %strategy.name(), pair = %strategy.pair(), "Registered strategy");
            strategies.push(strategy);
        }

        Self {
            strategies,
            price_history: HashMap::new(),
            max_history: Self::DEFAULT_MAX_HISTORY,
        }
    }

    /// Process one market event. Returns signals from all matching strategies.
    /// Only passes events to strategies configured for the event's pair.
    pub fn process(&mut self, event: &MarketEvent) -> Vec<Signal> {
        if event.is_candle_closed {
            let history = self
                .price_history
                .entry(event.pair.clone())
                .or_default();
            history.push(event.price);
            if history.len() > self.max_history {
                history.remove(0);
            }
        }

        let _history = self
            .price_history
            .get(&event.pair)
            .cloned()
            .unwrap_or_default();

        // Build a single-event slice for strategies that need the latest event
        let events_slice = std::slice::from_ref(event);

        self.strategies
            .iter()
            .filter(|s| s.pair() == event.pair)
            .filter_map(|s| {
                // Strategies receive the event slice; they can also use
                // historical data if they hold internal state.
                // Here we pass the current event as a single-element slice.
                s.evaluate(events_slice)
            })
            .collect()
    }

    /// Run the strategy dispatch loop.
    /// Reads from `market_rx`, pushes signals to `signal_tx`.
    /// Suppresses signals when engine is paused/halted.
    pub async fn run(
        mut self,
        mut market_rx: broadcast::Receiver<MarketEvent>,
        signal_tx: mpsc::Sender<Signal>,
        engine_state: Arc<tokio::sync::RwLock<EngineState>>,
    ) {
        info!("StrategyRegistry running");
        loop {
            match market_rx.recv().await {
                Ok(event) => {
                    let state = *engine_state.read().await;
                    if state != EngineState::Running {
                        continue; // suppress signals while paused/halted/stopped
                    }

                    let signals = self.process(&event);
                    for signal in signals {
                        if signal_tx.send(signal).await.is_err() {
                            warn!("Signal channel closed — stopping strategy registry");
                            return;
                        }
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!(dropped = n, "Strategy registry lagged — dropped market events");
                }
                Err(broadcast::error::RecvError::Closed) => {
                    warn!("Market broadcast channel closed");
                    return;
                }
            }
        }
    }
}

// ─── Strategy builders ────────────────────────────────────────────────────────

fn build_strategy(
    cfg: &StrategyConfig,
) -> Result<Box<dyn Strategy>, String> {
    match cfg.strategy_type.as_str() {
        "rsi" => {
            let period = param_usize(&cfg.params, "period", 14);
            let overbought = param_f64(&cfg.params, "overbought", 70.0);
            let oversold = param_f64(&cfg.params, "oversold", 30.0);
            Ok(Box::new(RsiStrategy::new(cfg.clone(), period, overbought, oversold)))
        }
        "macd" => {
            let fast = param_usize(&cfg.params, "fast", 12);
            let slow = param_usize(&cfg.params, "slow", 26);
            let signal = param_usize(&cfg.params, "signal", 9);
            Ok(Box::new(MacdStrategy::new(cfg.clone(), fast, slow, signal)))
        }
        other => Err(format!("unknown type '{other}'")),
    }
}

fn param_f64(params: &HashMap<String, toml::Value>, key: &str, default: f64) -> f64 {
    params
        .get(key)
        .and_then(|v| v.as_float())
        .unwrap_or(default)
}

fn param_usize(params: &HashMap<String, toml::Value>, key: &str, default: usize) -> usize {
    params
        .get(key)
        .and_then(|v| v.as_integer())
        .map(|v| v as usize)
        .unwrap_or(default)
}

// ─── Concrete strategy types ──────────────────────────────────────────────────

struct RsiStrategy {
    cfg: StrategyConfig,
    indicator: RsiIndicator,
    #[allow(dead_code)]
    history: Vec<f64>,
}

impl RsiStrategy {
    fn new(cfg: StrategyConfig, period: usize, overbought: f64, oversold: f64) -> Self {
        Self {
            cfg,
            indicator: RsiIndicator::new(period, overbought, oversold),
            history: Vec::new(),
        }
    }
}

impl Strategy for RsiStrategy {
    fn name(&self) -> &str {
        &self.cfg.name
    }

    fn pair(&self) -> &str {
        &self.cfg.pair
    }

    fn evaluate(&self, events: &[MarketEvent]) -> Option<Signal> {
        let closed_prices: Vec<f64> = events
            .iter()
            .filter(|e| e.is_candle_closed)
            .map(|e| e.price)
            .collect();

        if closed_prices.is_empty() {
            return None;
        }

        // For a full implementation, the registry passes accumulated history.
        // Here we use whatever closed prices arrived.
        let rsi = self.indicator.compute(&closed_prices)?;

        if rsi <= self.indicator.oversold {
            Some(Signal::Buy {
                pair: self.cfg.pair.clone(),
                quantity: self.cfg.quantity,
            })
        } else if rsi >= self.indicator.overbought {
            Some(Signal::Sell {
                pair: self.cfg.pair.clone(),
                quantity: self.cfg.quantity,
            })
        } else {
            None
        }
    }
}

struct MacdStrategy {
    cfg: StrategyConfig,
    indicator: MacdIndicator,
}

impl MacdStrategy {
    fn new(cfg: StrategyConfig, fast: usize, slow: usize, signal: usize) -> Self {
        Self {
            cfg,
            indicator: MacdIndicator::new(fast, slow, signal),
        }
    }
}

impl Strategy for MacdStrategy {
    fn name(&self) -> &str {
        &self.cfg.name
    }

    fn pair(&self) -> &str {
        &self.cfg.pair
    }

    fn evaluate(&self, events: &[MarketEvent]) -> Option<Signal> {
        let closes: Vec<f64> = events
            .iter()
            .filter(|e| e.is_candle_closed)
            .map(|e| e.price)
            .collect();

        use crate::indicators::macd::MacdSignal;
        match self.indicator.compute(&closes)? {
            MacdSignal::Bullish => Some(Signal::Buy {
                pair: self.cfg.pair.clone(),
                quantity: self.cfg.quantity,
            }),
            MacdSignal::Bearish => Some(Signal::Sell {
                pair: self.cfg.pair.clone(),
                quantity: self.cfg.quantity,
            }),
            MacdSignal::Neutral => None,
        }
    }
}
