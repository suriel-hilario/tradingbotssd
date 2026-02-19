pub mod config;
pub mod indicators;
pub mod registry;

pub use config::{StrategyConfig, StrategyFileConfig};
pub use registry::StrategyRegistry;

use common::{MarketEvent, Signal};

/// All strategy implementations must satisfy this trait.
pub trait Strategy: Send + Sync {
    /// Human-readable name of this strategy instance.
    fn name(&self) -> &str;

    /// The trading pair this strategy watches (e.g. "BTCUSDT").
    fn pair(&self) -> &str;

    /// Evaluate the latest batch of market events and optionally emit a signal.
    ///
    /// Only events where `is_candle_closed == true` should influence indicators.
    /// Returns `None` if no actionable signal is present.
    fn evaluate(&self, events: &[MarketEvent]) -> Option<Signal>;
}
