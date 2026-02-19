use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Live market data event from the exchange stream.
/// Emitted on every kline update (1-minute candles from Binance).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketEvent {
    pub pair: String,
    /// Latest close price of the current candle.
    pub price: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub volume: f64,
    /// True when the candle has closed (finalized). Indicators should only
    /// process events where `is_candle_closed == true`.
    pub is_candle_closed: bool,
    pub timestamp: DateTime<Utc>,
}

/// Side of a trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "UPPERCASE")]
#[sqlx(type_name = "TEXT", rename_all = "UPPERCASE")]
pub enum OrderSide {
    Buy,
    Sell,
}

impl std::fmt::Display for OrderSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrderSide::Buy => write!(f, "BUY"),
            OrderSide::Sell => write!(f, "SELL"),
        }
    }
}

/// An order to be submitted to the exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub pair: String,
    pub side: OrderSide,
    pub quantity: f64,
    /// `None` = market order; `Some(price)` = limit order.
    pub price: Option<f64>,
}

impl Order {
    pub fn market(pair: impl Into<String>, side: OrderSide, quantity: f64) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            pair: pair.into(),
            side,
            quantity,
            price: None,
        }
    }
}

/// Confirmation of a filled order returned by the exchange.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fill {
    pub order_id: String,
    pub pair: String,
    pub side: OrderSide,
    pub fill_price: f64,
    pub quantity: f64,
    pub timestamp: DateTime<Utc>,
}

/// Signal emitted by a strategy, passed to the Risk Manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Signal {
    Buy { pair: String, quantity: f64 },
    Sell { pair: String, quantity: f64 },
}

impl Signal {
    pub fn pair(&self) -> &str {
        match self {
            Signal::Buy { pair, .. } | Signal::Sell { pair, .. } => pair,
        }
    }

    pub fn quantity(&self) -> f64 {
        match self {
            Signal::Buy { quantity, .. } | Signal::Sell { quantity, .. } => *quantity,
        }
    }

    pub fn side(&self) -> OrderSide {
        match self {
            Signal::Buy { .. } => OrderSide::Buy,
            Signal::Sell { .. } => OrderSide::Sell,
        }
    }
}

/// An open trading position recorded in the database.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub id: String,
    pub pair: String,
    pub side: OrderSide,
    pub entry_price: f64,
    pub quantity: f64,
    pub mode: TradingMode,
    pub opened_at: DateTime<Utc>,
}

/// Whether the bot is running against the real exchange or simulating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(type_name = "TEXT", rename_all = "lowercase")]
pub enum TradingMode {
    Live,
    Paper,
}

impl std::fmt::Display for TradingMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TradingMode::Live => write!(f, "live"),
            TradingMode::Paper => write!(f, "paper"),
        }
    }
}

/// Reason an order was rejected by the Risk Manager.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RejectionReason {
    ExposureLimitExceeded,
    StopLossProximity,
    HardCeilingReached,
    DrawdownHalt,
    Other(String),
}

impl std::fmt::Display for RejectionReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RejectionReason::ExposureLimitExceeded => write!(f, "exposure limit exceeded"),
            RejectionReason::StopLossProximity => write!(f, "stop-loss proximity"),
            RejectionReason::HardCeilingReached => write!(f, "hard order ceiling reached"),
            RejectionReason::DrawdownHalt => write!(f, "max drawdown halt active"),
            RejectionReason::Other(s) => write!(f, "{s}"),
        }
    }
}

/// Current state of the trading engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum EngineState {
    #[default]
    Stopped,
    Running,
    Paused,
    Halted,
}

impl std::fmt::Display for EngineState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EngineState::Stopped => write!(f, "stopped"),
            EngineState::Running => write!(f, "running"),
            EngineState::Paused => write!(f, "paused"),
            EngineState::Halted => write!(f, "halted"),
        }
    }
}

/// Commands sent to the engine via the command channel.
#[derive(Debug, Clone)]
pub enum EngineCommand {
    Start,
    Stop,
    Pause,
    Resume,
    ResetDrawdown,
}

/// Events emitted by the Risk Manager.
#[derive(Debug, Clone)]
pub enum RiskEvent {
    OrderRejected {
        signal: Signal,
        reason: RejectionReason,
    },
    StopLossTriggered {
        pair: String,
        close_price: f64,
    },
    TakeProfitTriggered {
        pair: String,
        close_price: f64,
    },
    OrderFailed {
        pair: String,
        error: String,
    },
    DrawdownHaltEntered {
        drawdown_pct: f64,
    },
    DrawdownHaltExited,
}
