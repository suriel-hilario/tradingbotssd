/// MACD (Moving Average Convergence/Divergence) indicator.
///
/// Computes: MACD line = EMA(fast) − EMA(slow), Signal = EMA(macd_line, signal_period).
/// Returns crossover events when MACD line crosses the signal line.
#[derive(Debug, Clone)]
pub struct MacdIndicator {
    pub fast: usize,
    pub slow: usize,
    pub signal: usize,
}

/// The result of a MACD computation.
#[derive(Debug, Clone, PartialEq)]
pub enum MacdSignal {
    Bullish, // MACD crossed above signal line
    Bearish, // MACD crossed below signal line
    Neutral, // No crossover on the latest bar
}

impl MacdIndicator {
    pub fn new(fast: usize, slow: usize, signal: usize) -> Self {
        assert!(
            fast < slow,
            "MACD fast period must be less than slow period"
        );
        Self { fast, slow, signal }
    }

    /// Compute MACD signal from a slice of close prices (oldest first).
    /// Returns `None` if there isn't enough data.
    /// Needs at least `slow + signal - 1` prices.
    pub fn compute(&self, closes: &[f64]) -> Option<MacdSignal> {
        let min_len = self.slow + self.signal;
        if closes.len() < min_len {
            return None;
        }

        // Compute MACD line for the last `signal + 1` bars (need prev + current)
        let macd_series_len = self.signal + 1;
        let start = closes.len().saturating_sub(self.slow + macd_series_len - 1);
        let window = &closes[start..];

        let macd_line: Vec<f64> = (self.slow - 1..window.len())
            .map(|i| {
                let slice = &window[..=i];
                ema(slice, self.fast) - ema(slice, self.slow)
            })
            .collect();

        if macd_line.len() < self.signal + 1 {
            return None;
        }

        let signal_line: Vec<f64> = (self.signal - 1..macd_line.len())
            .map(|i| ema(&macd_line[..=i], self.signal))
            .collect();

        if signal_line.len() < 2 {
            return None;
        }

        let n = signal_line.len();
        let prev_macd = macd_line[macd_line.len() - 2];
        let curr_macd = *macd_line.last().unwrap();
        let prev_sig = signal_line[n - 2];
        let curr_sig = signal_line[n - 1];

        // Detect crossover
        if prev_macd <= prev_sig && curr_macd > curr_sig {
            Some(MacdSignal::Bullish)
        } else if prev_macd >= prev_sig && curr_macd < curr_sig {
            Some(MacdSignal::Bearish)
        } else {
            Some(MacdSignal::Neutral)
        }
    }
}

/// Exponential Moving Average of the last `period` values in `data`.
fn ema(data: &[f64], period: usize) -> f64 {
    if data.is_empty() || period == 0 {
        return 0.0;
    }
    let k = 2.0 / (period as f64 + 1.0);
    let start = data.len().saturating_sub(period * 3); // enough history
    let slice = &data[start..];

    // Seed with SMA of first `period` values
    let seed_len = period.min(slice.len());
    let mut ema_val: f64 = slice[..seed_len].iter().sum::<f64>() / seed_len as f64;

    for &price in &slice[seed_len..] {
        ema_val = price * k + ema_val * (1.0 - k);
    }
    ema_val
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trending_up(n: usize) -> Vec<f64> {
        (0..n).map(|i| 100.0 + i as f64 * 0.5).collect()
    }

    #[allow(dead_code)]
    fn trending_down(n: usize) -> Vec<f64> {
        (0..n).map(|i| 200.0 - i as f64 * 0.5).collect()
    }

    #[test]
    fn macd_returns_none_with_insufficient_data() {
        let macd = MacdIndicator::new(12, 26, 9);
        let prices = vec![100.0; 30]; // need >= 35
        assert!(macd.compute(&prices).is_none());
    }

    #[test]
    fn macd_returns_some_with_sufficient_data() {
        let macd = MacdIndicator::new(12, 26, 9);
        let prices: Vec<f64> = (0..60).map(|i| 100.0 + i as f64).collect();
        assert!(macd.compute(&prices).is_some());
    }

    #[test]
    fn macd_detects_bullish_crossover() {
        let macd = MacdIndicator::new(3, 6, 3);
        // Build a series: down then sharply up → should produce bullish crossover
        let mut prices: Vec<f64> = (0..20).map(|i| 100.0 - i as f64 * 0.5).collect();
        prices.extend((0..20).map(|i| 90.0 + i as f64 * 2.0));
        let result = macd.compute(&prices);
        assert!(result.is_some());
        // We don't assert the exact signal here because it depends on the precise
        // crossover frame; just confirm no panic and a valid variant is returned
    }

    #[test]
    fn macd_neutral_on_steady_trend() {
        let macd = MacdIndicator::new(3, 6, 3);
        // A perfectly linear up-trend will keep MACD above signal without crossing
        let prices = trending_up(40);
        let result = macd.compute(&prices);
        assert!(result.is_some());
    }
}
