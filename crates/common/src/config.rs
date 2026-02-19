use crate::TradingMode;

/// All configuration loaded from environment variables at startup.
/// Missing required variables cause an immediate panic with a clear message.
#[derive(Debug, Clone)]
pub struct Config {
    // Exchange credentials
    pub binance_api_key: String,
    pub binance_secret: String,

    // Telegram
    pub telegram_token: String,
    pub telegram_allowed_user_ids: Vec<i64>,

    // Dashboard
    pub dashboard_token: String,
    pub dashboard_port: u16,

    // Trading
    pub trading_mode: TradingMode,
    pub paper_slippage_bps: f64,

    // Database
    pub database_url: String,

    // Strategy config file path
    pub strategy_config_path: String,
}

impl Config {
    /// Load all configuration from environment variables.
    /// Loads `.env` if present. Panics on any missing required variable.
    pub fn from_env() -> Self {
        let _ = dotenvy::dotenv(); // ignore error if .env not present

        let trading_mode = match required_env("TRADING_MODE").to_lowercase().as_str() {
            "paper" => TradingMode::Paper,
            "live" => TradingMode::Live,
            other => panic!(
                "ERROR: TRADING_MODE must be 'paper' or 'live', got: '{other}'"
            ),
        };

        let telegram_allowed_user_ids = required_env("TELEGRAM_ALLOWED_USER_IDS")
            .split(',')
            .map(|s| {
                s.trim()
                    .parse::<i64>()
                    .unwrap_or_else(|_| {
                        panic!(
                            "TELEGRAM_ALLOWED_USER_IDS contains non-numeric ID: '{}'",
                            s.trim()
                        )
                    })
            })
            .collect();

        Config {
            binance_api_key: required_env("BINANCE_API_KEY"),
            binance_secret: required_env("BINANCE_SECRET"),
            telegram_token: required_env("TELEGRAM_TOKEN"),
            telegram_allowed_user_ids,
            dashboard_token: required_env("DASHBOARD_TOKEN"),
            dashboard_port: optional_env("DASHBOARD_PORT")
                .and_then(|v| v.parse().ok())
                .unwrap_or(8080),
            trading_mode,
            paper_slippage_bps: optional_env("PAPER_SLIPPAGE_BPS")
                .and_then(|v| v.parse().ok())
                .unwrap_or(10.0),
            database_url: required_env("DATABASE_URL"),
            strategy_config_path: optional_env("STRATEGY_CONFIG_PATH")
                .unwrap_or_else(|| "config/strategies.toml".to_string()),
        }
    }
}

fn required_env(key: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| {
        panic!("Required environment variable '{key}' is not set. Check your .env file.")
    })
}

fn optional_env(key: &str) -> Option<String> {
    std::env::var(key).ok()
}
