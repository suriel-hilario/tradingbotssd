use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Top-level strategy config file (TOML).
///
/// Example `config/strategies.toml`:
/// ```toml
/// [[strategy]]
/// type = "rsi"
/// name = "BTC RSI 14"
/// pair = "BTCUSDT"
/// quantity = 0.001
///
/// [strategy.params]
/// period = 14
/// overbought = 70.0
/// oversold = 30.0
/// ```
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StrategyFileConfig {
    #[serde(rename = "strategy")]
    pub strategies: Vec<StrategyConfig>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StrategyConfig {
    /// Strategy type identifier: "rsi" or "macd".
    #[serde(rename = "type")]
    pub strategy_type: String,
    /// Human-readable name shown in logs and dashboard.
    pub name: String,
    /// Trading pair, e.g. "BTCUSDT".
    pub pair: String,
    /// Order quantity in base asset units.
    pub quantity: f64,
    /// Indicator-specific parameters.
    #[serde(default)]
    pub params: HashMap<String, toml::Value>,
}

impl StrategyFileConfig {
    /// Load from a TOML file. Exits process on error.
    pub fn load(path: &str) -> Self {
        let content = std::fs::read_to_string(path).unwrap_or_else(|e| {
            panic!("Failed to read strategy config at '{path}': {e}")
        });
        toml::from_str(&content).unwrap_or_else(|e| {
            panic!("Failed to parse strategy config at '{path}': {e}")
        })
    }
}
