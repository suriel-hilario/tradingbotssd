use std::time::{SystemTime, UNIX_EPOCH};

use async_trait::async_trait;
use chrono::Utc;
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::Deserialize;
use sha2::Sha256;
use tracing::debug;

use common::{Error, ExchangeClient, Fill, Order, OrderSide, Position, Result, TradingMode};

const BASE_URL: &str = "https://api.binance.com";

/// REST API client for Binance. Used for order placement and account queries.
pub struct BinanceClient {
    api_key: String,
    secret: String,
    http: Client,
}

impl BinanceClient {
    pub fn new(api_key: impl Into<String>, secret: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            secret: secret.into(),
            http: Client::builder()
                .use_rustls_tls()
                .build()
                .expect("Failed to build HTTP client"),
        }
    }

    fn timestamp_ms() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64
    }

    fn sign(&self, query: &str) -> String {
        type HmacSha256 = Hmac<Sha256>;
        let mut mac = HmacSha256::new_from_slice(self.secret.as_bytes())
            .expect("HMAC accepts any key length");
        mac.update(query.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    async fn signed_get(&self, path: &str, params: &str) -> Result<String> {
        let ts = Self::timestamp_ms();
        let query = format!("{params}&timestamp={ts}");
        let signature = self.sign(&query);
        let url = format!("{BASE_URL}{path}?{query}&signature={signature}");

        let resp = self
            .http
            .get(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await
            .map_err(|e| Error::Http(e.to_string()))?;

        let status = resp.status();
        let body = resp.text().await.map_err(|e| Error::Http(e.to_string()))?;

        if !status.is_success() {
            return Err(Error::Exchange(format!("HTTP {status}: {body}")));
        }
        Ok(body)
    }

    async fn signed_post(&self, path: &str, params: &str) -> Result<String> {
        let ts = Self::timestamp_ms();
        let query = format!("{params}&timestamp={ts}");
        let signature = self.sign(&query);
        let body = format!("{query}&signature={signature}");
        let url = format!("{BASE_URL}{path}");

        let resp = self
            .http
            .post(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(body)
            .send()
            .await
            .map_err(|e| Error::Http(e.to_string()))?;

        let status = resp.status();
        let text = resp.text().await.map_err(|e| Error::Http(e.to_string()))?;

        if !status.is_success() {
            return Err(Error::Exchange(format!("HTTP {status}: {text}")));
        }
        Ok(text)
    }
}

#[async_trait]
impl ExchangeClient for BinanceClient {
    async fn submit_order(&self, order: &Order) -> Result<Fill> {
        let side = order.side.to_string();
        let order_type = if order.price.is_some() {
            "LIMIT"
        } else {
            "MARKET"
        };

        let mut params = format!(
            "symbol={}&side={}&type={}&quantity={}",
            order.pair, side, order_type, order.quantity
        );
        if let Some(price) = order.price {
            params.push_str(&format!("&price={}&timeInForce=GTC", price));
        }

        debug!(pair = %order.pair, side = %side, "Submitting order to Binance");
        let body = self.signed_post("/api/v3/order", &params).await?;

        let resp: OrderResponse =
            serde_json::from_str(&body).map_err(|e| Error::Exchange(e.to_string()))?;

        let fill_price = resp
            .fills
            .first()
            .and_then(|f| f.price.parse::<f64>().ok())
            .unwrap_or_else(|| order.price.unwrap_or(0.0));

        Ok(Fill {
            order_id: resp.client_order_id,
            pair: order.pair.clone(),
            side: order.side,
            fill_price,
            quantity: order.quantity,
            timestamp: Utc::now(),
        })
    }

    async fn open_positions(&self) -> Result<Vec<Position>> {
        // Fetch account info and extract non-zero balances as pseudo-positions.
        // For a more accurate implementation, query open orders or use futures API.
        let body = self.signed_get("/api/v3/account", "").await?;
        let account: AccountResponse =
            serde_json::from_str(&body).map_err(|e| Error::Exchange(e.to_string()))?;

        let positions = account
            .balances
            .into_iter()
            .filter(|b| {
                b.free.parse::<f64>().unwrap_or(0.0) > 0.0
                    || b.locked.parse::<f64>().unwrap_or(0.0) > 0.0
            })
            .filter(|b| b.asset != "USDT" && b.asset != "BNB")
            .map(|b| {
                let qty =
                    b.free.parse::<f64>().unwrap_or(0.0) + b.locked.parse::<f64>().unwrap_or(0.0);
                Position {
                    id: uuid::Uuid::new_v4().to_string(),
                    pair: format!("{}USDT", b.asset),
                    side: OrderSide::Buy,
                    entry_price: 0.0, // unknown without trade history
                    quantity: qty,
                    mode: TradingMode::Live,
                    opened_at: Utc::now(),
                }
            })
            .collect();

        Ok(positions)
    }

    async fn current_price(&self, pair: &str) -> Result<f64> {
        let url = format!("{BASE_URL}/api/v3/ticker/price?symbol={pair}");
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| Error::Http(e.to_string()))?;

        let ticker: PriceTicker = resp.json().await.map_err(|e| Error::Http(e.to_string()))?;

        ticker
            .price
            .parse::<f64>()
            .map_err(|e| Error::Exchange(e.to_string()))
    }
}

// ─── Response types ───────────────────────────────────────────────────────────

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct OrderResponse {
    client_order_id: String,
    #[serde(default)]
    fills: Vec<FillDetail>,
}

#[derive(Deserialize)]
struct FillDetail {
    price: String,
}

#[derive(Deserialize)]
struct AccountResponse {
    balances: Vec<Balance>,
}

#[derive(Deserialize)]
struct Balance {
    asset: String,
    free: String,
    locked: String,
}

#[derive(Deserialize)]
struct PriceTicker {
    price: String,
}
