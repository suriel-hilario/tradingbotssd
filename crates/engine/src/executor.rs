use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::mpsc;
use tracing::{error, info, warn};

use common::{ExchangeClient, Fill, Order, RiskEvent, TradingMode};

/// Receives approved orders from the Risk Manager and submits them to the exchange.
/// On success, persists the fill to the database.
///
/// This is the ONLY component that calls `ExchangeClient::submit_order`.
pub struct OrderExecutor {
    order_rx: mpsc::Receiver<Order>,
    risk_event_tx: mpsc::Sender<RiskEvent>,
    client: Arc<dyn ExchangeClient>,
    db: SqlitePool,
    mode: TradingMode,
}

impl OrderExecutor {
    pub fn new(
        order_rx: mpsc::Receiver<Order>,
        risk_event_tx: mpsc::Sender<RiskEvent>,
        client: Arc<dyn ExchangeClient>,
        db: SqlitePool,
        mode: TradingMode,
    ) -> Self {
        Self {
            order_rx,
            risk_event_tx,
            client,
            db,
            mode,
        }
    }

    /// Run the executor loop. Call from `tokio::spawn`.
    pub async fn run(mut self) {
        info!("OrderExecutor running in {:?} mode", self.mode);
        while let Some(order) = self.order_rx.recv().await {
            info!(pair = %order.pair, side = ?order.side, qty = order.quantity, "Executing order");

            match self.client.submit_order(&order).await {
                Ok(fill) => {
                    info!(
                        pair = %fill.pair,
                        price = fill.fill_price,
                        qty = fill.quantity,
                        "Order filled"
                    );
                    if let Err(e) = self.persist_fill(&fill).await {
                        error!("Failed to persist fill: {e}");
                    }
                }
                Err(e) => {
                    error!(pair = %order.pair, error = %e, "Order submission failed");
                    let _ = self
                        .risk_event_tx
                        .send(RiskEvent::OrderFailed {
                            pair: order.pair.clone(),
                            error: e.to_string(),
                        })
                        .await;
                }
            }
        }
        warn!("OrderExecutor: order channel closed");
    }

    async fn persist_fill(&self, fill: &Fill) -> Result<(), sqlx::Error> {
        let side = fill.side.to_string();
        let mode = self.mode.to_string();
        let opened_at = fill.timestamp.to_rfc3339();

        sqlx::query!(
            r#"
            INSERT INTO positions (id, pair, side, entry_price, quantity, mode, opened_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            ON CONFLICT(id) DO NOTHING
            "#,
            fill.order_id,
            fill.pair,
            side,
            fill.fill_price,
            fill.quantity,
            mode,
            opened_at,
        )
        .execute(&self.db)
        .await?;

        Ok(())
    }
}
