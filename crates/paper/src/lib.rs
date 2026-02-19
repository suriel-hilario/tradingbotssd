use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use tokio::sync::RwLock;
use tracing::{debug, info};

use common::{Error, ExchangeClient, Fill, Order, OrderSide, Position, Result, TradingMode};

/// Simulated exchange client for paper trading.
///
/// Fills are simulated at the latest known price with configurable slippage.
/// No real orders are ever sent to Binance.
pub struct PaperClient {
    /// Simulated balance in USDT.
    #[allow(dead_code)]
    balance_usd: Arc<RwLock<f64>>,
    /// Open simulated positions, keyed by position ID.
    positions: Arc<RwLock<Vec<Position>>>,
    /// Latest known price per pair, updated via `update_price`.
    prices: Arc<RwLock<HashMap<String, f64>>>,
    /// Slippage in basis points applied to all fills.
    slippage_bps: f64,
}

impl PaperClient {
    pub fn new(initial_balance_usd: f64, slippage_bps: f64) -> Self {
        info!(
            balance = initial_balance_usd,
            slippage_bps = slippage_bps,
            "PaperClient initialized"
        );
        Self {
            balance_usd: Arc::new(RwLock::new(initial_balance_usd)),
            positions: Arc::new(RwLock::new(Vec::new())),
            prices: Arc::new(RwLock::new(HashMap::new())),
            slippage_bps,
        }
    }

    /// Update the latest price for a pair (called by the market event loop).
    pub async fn update_price(&self, pair: &str, price: f64) {
        self.prices.write().await.insert(pair.to_string(), price);
    }

    /// Expose open positions (for the dashboard API and auditing).
    pub fn positions_handle(&self) -> Arc<RwLock<Vec<Position>>> {
        self.positions.clone()
    }
}

#[async_trait]
impl ExchangeClient for PaperClient {
    async fn submit_order(&self, order: &Order) -> Result<Fill> {
        let prices = self.prices.read().await;
        let mid_price = prices.get(&order.pair).copied().ok_or_else(|| {
            Error::Exchange(format!(
                "PaperClient has no price for pair '{}'. Ensure market events are flowing.",
                order.pair
            ))
        })?;
        drop(prices);

        // Apply slippage: buys pay more, sells receive less
        let fill_price = match order.side {
            OrderSide::Buy => mid_price * (1.0 + self.slippage_bps / 10_000.0),
            OrderSide::Sell => mid_price * (1.0 - self.slippage_bps / 10_000.0),
        };

        debug!(
            pair = %order.pair,
            side = ?order.side,
            mid = mid_price,
            fill = fill_price,
            qty = order.quantity,
            "Paper fill simulated"
        );

        let fill = Fill {
            order_id: order.id.clone(),
            pair: order.pair.clone(),
            side: order.side,
            fill_price,
            quantity: order.quantity,
            timestamp: Utc::now(),
        };

        // Update in-memory position ledger
        let mut positions = self.positions.write().await;
        match order.side {
            OrderSide::Buy => {
                positions.push(Position {
                    id: order.id.clone(),
                    pair: order.pair.clone(),
                    side: OrderSide::Buy,
                    entry_price: fill_price,
                    quantity: order.quantity,
                    mode: TradingMode::Paper,
                    opened_at: Utc::now(),
                });
            }
            OrderSide::Sell => {
                // Remove the first matching open buy position
                if let Some(idx) = positions.iter().position(|p| p.pair == order.pair) {
                    positions.remove(idx);
                }
            }
        }

        Ok(fill)
    }

    async fn open_positions(&self) -> Result<Vec<Position>> {
        Ok(self.positions.read().await.clone())
    }

    async fn current_price(&self, pair: &str) -> Result<f64> {
        self.prices
            .read()
            .await
            .get(pair)
            .copied()
            .ok_or_else(|| Error::Exchange(format!("No price available for {pair}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use common::Order;

    #[tokio::test]
    async fn paper_buy_fill_applies_positive_slippage() {
        let client = PaperClient::new(10_000.0, 10.0); // 10 bps
        client.update_price("BTCUSDT", 1000.0).await;

        let order = Order::market("BTCUSDT", OrderSide::Buy, 0.01);
        let fill = client.submit_order(&order).await.unwrap();

        let expected = 1000.0 * (1.0 + 10.0 / 10_000.0);
        assert!(
            (fill.fill_price - expected).abs() < 1e-6,
            "Buy fill price {}, expected {}",
            fill.fill_price,
            expected
        );
    }

    #[tokio::test]
    async fn paper_sell_fill_applies_negative_slippage() {
        let client = PaperClient::new(10_000.0, 10.0);
        client.update_price("BTCUSDT", 1000.0).await;

        // First buy, then sell
        let buy = Order::market("BTCUSDT", OrderSide::Buy, 0.01);
        client.submit_order(&buy).await.unwrap();

        let sell = Order::market("BTCUSDT", OrderSide::Sell, 0.01);
        let fill = client.submit_order(&sell).await.unwrap();

        let expected = 1000.0 * (1.0 - 10.0 / 10_000.0);
        assert!(
            (fill.fill_price - expected).abs() < 1e-6,
            "Sell fill price {}, expected {}",
            fill.fill_price,
            expected
        );
    }

    #[tokio::test]
    async fn paper_position_recorded_after_buy() {
        let client = PaperClient::new(10_000.0, 0.0);
        client.update_price("ETHUSDT", 500.0).await;

        let order = Order::market("ETHUSDT", OrderSide::Buy, 1.0);
        client.submit_order(&order).await.unwrap();

        let positions = client.open_positions().await.unwrap();
        assert_eq!(positions.len(), 1);
        assert_eq!(positions[0].pair, "ETHUSDT");
        assert_eq!(positions[0].mode, TradingMode::Paper);
    }

    #[tokio::test]
    async fn paper_position_removed_after_sell() {
        let client = PaperClient::new(10_000.0, 0.0);
        client.update_price("ETHUSDT", 500.0).await;

        let buy = Order::market("ETHUSDT", OrderSide::Buy, 1.0);
        client.submit_order(&buy).await.unwrap();

        let sell = Order::market("ETHUSDT", OrderSide::Sell, 1.0);
        client.submit_order(&sell).await.unwrap();

        let positions = client.open_positions().await.unwrap();
        assert!(positions.is_empty());
    }
}
