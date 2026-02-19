use std::time::Duration;

use chrono::{DateTime, TimeZone, Utc};
use futures_util::StreamExt;
use serde::Deserialize;
use tokio::sync::broadcast;
use tokio_tungstenite::connect_async;
use tracing::{info, warn};
use url::Url;

use common::{MarketEvent, Result};

/// Binance kline/candlestick WebSocket stream for a single pair.
///
/// Connects to Binance's 1-minute kline stream, parses events into
/// `MarketEvent`, and publishes them on a broadcast channel.
/// Reconnects automatically with exponential backoff.
pub struct BinanceStream {
    pair: String,
    market_tx: broadcast::Sender<MarketEvent>,
}

impl BinanceStream {
    pub fn new(pair: impl Into<String>, market_tx: broadcast::Sender<MarketEvent>) -> Self {
        Self {
            pair: pair.into(),
            market_tx,
        }
    }

    /// Run the stream loop forever, reconnecting on failure.
    /// Call this inside a `tokio::spawn`.
    pub async fn run(self) {
        let mut backoff = Duration::from_secs(1);
        const MAX_BACKOFF: Duration = Duration::from_secs(60);

        loop {
            info!(pair = %self.pair, "Connecting to Binance WebSocket stream");
            match self.connect_once().await {
                Ok(()) => {
                    info!(pair = %self.pair, "WebSocket stream closed cleanly");
                    // Clean close — reconnect after a short delay (e.g. 24h session end)
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    backoff = Duration::from_secs(1);
                }
                Err(e) => {
                    warn!(pair = %self.pair, error = %e, backoff = ?backoff, "WebSocket error, reconnecting");
                    tokio::time::sleep(backoff).await;
                    backoff = (backoff * 2).min(MAX_BACKOFF);
                }
            }
        }
    }

    async fn connect_once(&self) -> Result<()> {
        let pair_lower = self.pair.to_lowercase();
        // Subscribe to 1-minute kline stream
        let url_str = format!(
            "wss://stream.binance.com:9443/ws/{}@kline_1m",
            pair_lower
        );
        let url = Url::parse(&url_str)
            .map_err(|e| common::Error::WebSocket(e.to_string()))?;

        let (ws_stream, _) = connect_async(url)
            .await
            .map_err(|e| common::Error::WebSocket(e.to_string()))?;

        let (_, mut read) = ws_stream.split();

        while let Some(msg) = read.next().await {
            let msg = msg.map_err(|e| common::Error::WebSocket(e.to_string()))?;

            if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                match parse_kline_event(&self.pair, &text) {
                    Ok(Some(event)) => {
                        // Ignore send errors (no active receivers)
                        let _ = self.market_tx.send(event);
                    }
                    Ok(None) => {} // non-kline message, skip
                    Err(e) => {
                        warn!(error = %e, "Failed to parse kline event");
                    }
                }
            }
        }

        Ok(())
    }
}

// ─── Binance kline JSON parsing ──────────────────────────────────────────────

#[derive(Deserialize)]
struct KlineWrapper {
    k: KlineData,
}

#[derive(Deserialize)]
struct KlineData {
    #[serde(rename = "o")]
    open: String,
    #[serde(rename = "h")]
    high: String,
    #[serde(rename = "l")]
    low: String,
    #[serde(rename = "c")]
    close: String,
    #[serde(rename = "v")]
    volume: String,
    #[serde(rename = "x")]
    is_closed: bool,
    #[serde(rename = "T")]
    close_time_ms: i64,
}

fn parse_kline_event(pair: &str, text: &str) -> Result<Option<MarketEvent>> {
    // Kline messages have an "e" field set to "kline"
    let wrapper: serde_json::Value = serde_json::from_str(text)?;
    if wrapper.get("e").and_then(|v| v.as_str()) != Some("kline") {
        return Ok(None);
    }

    let kline: KlineWrapper = serde_json::from_value(wrapper)?;
    let k = kline.k;

    let timestamp: DateTime<Utc> = Utc
        .timestamp_millis_opt(k.close_time_ms)
        .single()
        .unwrap_or_else(Utc::now);

    Ok(Some(MarketEvent {
        pair: pair.to_string(),
        price: k.close.parse().unwrap_or(0.0),
        open: k.open.parse().unwrap_or(0.0),
        high: k.high.parse().unwrap_or(0.0),
        low: k.low.parse().unwrap_or(0.0),
        volume: k.volume.parse().unwrap_or(0.0),
        is_candle_closed: k.is_closed,
        timestamp,
    }))
}
