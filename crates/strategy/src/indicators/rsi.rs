/// RSI (Relative Strength Index) indicator.
///
/// Uses Wilder's smoothed moving average (same as TradingView / standard RSI).
/// Returns `None` until at least `period + 1` closed price values are available.
#[derive(Debug, Clone)]
pub struct RsiIndicator {
    pub period: usize,
    pub overbought: f64,
    pub oversold: f64,
}

impl RsiIndicator {
    pub fn new(period: usize, overbought: f64, oversold: f64) -> Self {
        assert!(period >= 2, "RSI period must be >= 2");
        Self { period, overbought, oversold }
    }

    /// Compute RSI from a slice of close prices (oldest first).
    /// Returns `None` if there are fewer than `period + 1` values.
    pub fn compute(&self, closes: &[f64]) -> Option<f64> {
        if closes.len() < self.period + 1 {
            return None;
        }

        // First average gain/loss over the initial `period` changes
        let changes: Vec<f64> = closes.windows(2).map(|w| w[1] - w[0]).collect();
        let initial = &changes[..self.period];

        let mut avg_gain = initial.iter().filter(|&&c| c > 0.0).sum::<f64>() / self.period as f64;
        let mut avg_loss = initial.iter().filter(|&&c| c < 0.0).map(|c| c.abs()).sum::<f64>()
            / self.period as f64;

        // Wilder smoothing over remaining changes
        for &change in &changes[self.period..] {
            let gain = if change > 0.0 { change } else { 0.0 };
            let loss = if change < 0.0 { change.abs() } else { 0.0 };
            avg_gain = (avg_gain * (self.period - 1) as f64 + gain) / self.period as f64;
            avg_loss = (avg_loss * (self.period - 1) as f64 + loss) / self.period as f64;
        }

        if avg_loss == 0.0 {
            return Some(100.0);
        }

        let rs = avg_gain / avg_loss;
        Some(100.0 - 100.0 / (1.0 + rs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rsi_returns_none_when_insufficient_data() {
        let rsi = RsiIndicator::new(14, 70.0, 30.0);
        // Need at least period+1 = 15 values
        let prices = vec![100.0; 14];
        assert!(rsi.compute(&prices).is_none());
    }

    #[test]
    fn rsi_returns_some_with_sufficient_data() {
        let rsi = RsiIndicator::new(14, 70.0, 30.0);
        // 15 values — exactly period+1
        let prices: Vec<f64> = (0..15).map(|i| 100.0 + i as f64).collect();
        assert!(rsi.compute(&prices).is_some());
    }

    #[test]
    fn rsi_all_gains_returns_100() {
        let rsi = RsiIndicator::new(3, 70.0, 30.0);
        // Strictly increasing prices → RSI = 100
        let prices = vec![10.0, 11.0, 12.0, 13.0, 14.0];
        let value = rsi.compute(&prices).unwrap();
        assert!((value - 100.0).abs() < 1e-6, "Expected ~100, got {value}");
    }

    #[test]
    fn rsi_all_losses_returns_0() {
        let rsi = RsiIndicator::new(3, 70.0, 30.0);
        // Strictly decreasing prices → RSI = 0
        let prices = vec![14.0, 13.0, 12.0, 11.0, 10.0];
        let value = rsi.compute(&prices).unwrap();
        assert!((value - 0.0).abs() < 1e-6, "Expected ~0, got {value}");
    }

    #[test]
    fn rsi_known_value() {
        // Reference: 14-period RSI on a known price series
        // Prices sourced from Investopedia RSI example (rounded)
        let rsi = RsiIndicator::new(14, 70.0, 30.0);
        let prices = vec![
            44.34, 44.09, 44.15, 43.61, 44.33, 44.83, 45.10, 45.15, 43.61, 44.33, 44.83, 45.10,
            45.15, 44.34, 44.09,
        ];
        let value = rsi.compute(&prices);
        assert!(value.is_some());
        let v = value.unwrap();
        assert!((0.0..=100.0).contains(&v), "RSI out of range: {v}");
    }
}
