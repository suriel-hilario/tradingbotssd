use async_trait::async_trait;

use crate::{Fill, Order, Position, Result};

/// Abstraction over the exchange connection.
///
/// `BinanceClient` implements this for live trading.
/// `PaperClient` implements this for simulation.
///
/// Only `OrderExecutor` in `crates/engine` should hold a reference to a
/// `dyn ExchangeClient`. All order flow must go through the Risk Manager
/// before reaching the executor.
#[async_trait]
pub trait ExchangeClient: Send + Sync {
    /// Submit an order and return the fill confirmation.
    async fn submit_order(&self, order: &Order) -> Result<Fill>;

    /// Query currently open positions from the exchange.
    async fn open_positions(&self) -> Result<Vec<Position>>;

    /// Get the latest price for a trading pair.
    async fn current_price(&self, pair: &str) -> Result<f64>;
}
