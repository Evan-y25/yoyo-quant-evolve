//! Technical indicators for price analysis.
//!
//! Calculated from price data arrays. These are the building blocks
//! of technical analysis — moving averages, momentum, etc.

/// Calculate Simple Moving Average (SMA) for a given period.
/// Returns None if there aren't enough data points.
pub fn sma(prices: &[f64], period: usize) -> Option<f64> {
    if prices.len() < period || period == 0 {
        return None;
    }
    let sum: f64 = prices[prices.len() - period..].iter().sum();
    Some(sum / period as f64)
}

/// Calculate Exponential Moving Average (EMA) for a given period.
/// Uses the entire price history to compute the EMA.
/// Returns None if there aren't enough data points.
pub fn ema(prices: &[f64], period: usize) -> Option<f64> {
    if prices.len() < period || period == 0 {
        return None;
    }
    let multiplier = 2.0 / (period as f64 + 1.0);

    // Start with SMA of first `period` prices
    let initial_sma: f64 = prices[..period].iter().sum::<f64>() / period as f64;

    let mut ema_val = initial_sma;
    for &price in &prices[period..] {
        ema_val = (price - ema_val) * multiplier + ema_val;
    }

    Some(ema_val)
}

/// Calculate Relative Strength Index (RSI) for a given period (typically 14).
/// Returns a value between 0-100.
/// - Above 70: overbought
/// - Below 30: oversold
/// Returns None if there aren't enough data points.
pub fn rsi(prices: &[f64], period: usize) -> Option<f64> {
    if prices.len() < period + 1 || period == 0 {
        return None;
    }

    // Calculate price changes
    let changes: Vec<f64> = prices.windows(2).map(|w| w[1] - w[0]).collect();

    if changes.len() < period {
        return None;
    }

    // Initial average gain/loss from first `period` changes
    let mut avg_gain: f64 = changes[..period]
        .iter()
        .filter(|&&c| c > 0.0)
        .sum::<f64>()
        / period as f64;

    let mut avg_loss: f64 = changes[..period]
        .iter()
        .filter(|&&c| c < 0.0)
        .map(|c| c.abs())
        .sum::<f64>()
        / period as f64;

    // Smoothed RSI using Wilder's method
    for &change in &changes[period..] {
        let gain = if change > 0.0 { change } else { 0.0 };
        let loss = if change < 0.0 { change.abs() } else { 0.0 };

        avg_gain = (avg_gain * (period as f64 - 1.0) + gain) / period as f64;
        avg_loss = (avg_loss * (period as f64 - 1.0) + loss) / period as f64;
    }

    if avg_loss == 0.0 {
        return Some(100.0);
    }

    let rs = avg_gain / avg_loss;
    Some(100.0 - (100.0 / (1.0 + rs)))
}

/// Interpret RSI value as a human-readable signal.
pub fn rsi_signal(rsi_value: f64) -> &'static str {
    if rsi_value >= 80.0 {
        "🔴 Strongly Overbought"
    } else if rsi_value >= 70.0 {
        "🟠 Overbought"
    } else if rsi_value >= 60.0 {
        "🟡 Mildly Bullish"
    } else if rsi_value >= 40.0 {
        "⚪ Neutral"
    } else if rsi_value >= 30.0 {
        "🟡 Mildly Bearish"
    } else if rsi_value >= 20.0 {
        "🟠 Oversold"
    } else {
        "🟢 Strongly Oversold"
    }
}

/// Determine trend signal from SMA crossover (short SMA vs long SMA relative to current price).
pub fn sma_signal(price: f64, short_sma: f64, long_sma: f64) -> &'static str {
    if price > short_sma && short_sma > long_sma {
        "🟢 Bullish (price > short SMA > long SMA)"
    } else if price < short_sma && short_sma < long_sma {
        "🔴 Bearish (price < short SMA < long SMA)"
    } else if price > short_sma && short_sma < long_sma {
        "🟡 Recovery (price above short SMA, but still below long)"
    } else {
        "🟡 Mixed signals"
    }
}

/// MACD result containing the three components traders look at.
#[derive(Debug, Clone)]
pub struct MacdResult {
    /// MACD line: EMA(fast) - EMA(slow)
    pub macd_line: f64,
    /// Signal line: EMA(signal_period) of MACD line
    pub signal_line: f64,
    /// Histogram: MACD line - Signal line
    pub histogram: f64,
}

/// Calculate MACD (Moving Average Convergence Divergence).
///
/// Standard parameters: fast=12, slow=26, signal=9.
/// Returns None if there aren't enough data points (need at least `slow` prices).
///
/// The MACD is one of the most popular momentum indicators:
/// - MACD line crossing above signal line → bullish
/// - MACD line crossing below signal line → bearish
/// - Histogram growing → momentum increasing
/// - Histogram shrinking → momentum fading
pub fn macd(prices: &[f64], fast: usize, slow: usize, signal_period: usize) -> Option<MacdResult> {
    if prices.len() < slow || fast == 0 || slow == 0 || signal_period == 0 || fast >= slow {
        return None;
    }

    // Calculate MACD line at each point: EMA(fast) - EMA(slow)
    // We need enough points to compute both EMAs and then the signal EMA
    let multiplier_fast = 2.0 / (fast as f64 + 1.0);
    let multiplier_slow = 2.0 / (slow as f64 + 1.0);

    // For the signal line, we need a series of MACD values.
    // Build MACD line series starting from index `slow-1` (first point where both EMAs exist).
    let mut macd_series = Vec::new();

    // Re-compute from the start to get the MACD series
    // Fast EMA starts at index fast, slow EMA starts at index slow
    // We begin tracking MACD from index slow (when both exist)

    // Compute fast EMA up to index slow-1
    let mut fast_ema_series = prices[..fast].iter().sum::<f64>() / fast as f64;
    for &price in &prices[fast..slow] {
        fast_ema_series = (price - fast_ema_series) * multiplier_fast + fast_ema_series;
    }
    let mut slow_ema_series = prices[..slow].iter().sum::<f64>() / slow as f64;

    // First MACD value
    macd_series.push(fast_ema_series - slow_ema_series);

    // Continue from index slow onward
    for &price in &prices[slow..] {
        fast_ema_series = (price - fast_ema_series) * multiplier_fast + fast_ema_series;
        slow_ema_series = (price - slow_ema_series) * multiplier_slow + slow_ema_series;
        macd_series.push(fast_ema_series - slow_ema_series);
    }

    if macd_series.len() < signal_period {
        return None;
    }

    // Calculate signal line: EMA of MACD series
    let multiplier_signal = 2.0 / (signal_period as f64 + 1.0);
    let initial_signal: f64 = macd_series[..signal_period].iter().sum::<f64>() / signal_period as f64;
    let mut signal = initial_signal;
    for &m in &macd_series[signal_period..] {
        signal = (m - signal) * multiplier_signal + signal;
    }

    let macd_line = *macd_series.last().unwrap();
    let histogram = macd_line - signal;

    Some(MacdResult {
        macd_line,
        signal_line: signal,
        histogram,
    })
}

/// Interpret MACD as a human-readable signal.
pub fn macd_signal(result: &MacdResult) -> &'static str {
    if result.macd_line > result.signal_line && result.histogram > 0.0 {
        if result.macd_line > 0.0 {
            "🟢 Bullish (MACD above signal, both positive)"
        } else {
            "🟡 Turning bullish (MACD crossing above signal)"
        }
    } else if result.macd_line < result.signal_line && result.histogram < 0.0 {
        if result.macd_line < 0.0 {
            "🔴 Bearish (MACD below signal, both negative)"
        } else {
            "🟡 Turning bearish (MACD crossing below signal)"
        }
    } else {
        "⚪ Neutral (near crossover)"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sma_basic() {
        let prices = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        assert_eq!(sma(&prices, 3), Some(40.0)); // (30+40+50)/3
        assert_eq!(sma(&prices, 5), Some(30.0)); // (10+20+30+40+50)/5
    }

    #[test]
    fn test_sma_insufficient_data() {
        let prices = vec![10.0, 20.0];
        assert_eq!(sma(&prices, 5), None);
    }

    #[test]
    fn test_sma_zero_period() {
        let prices = vec![10.0, 20.0, 30.0];
        assert_eq!(sma(&prices, 0), None);
    }

    #[test]
    fn test_ema_basic() {
        let prices = vec![10.0, 20.0, 30.0, 40.0, 50.0];
        let result = ema(&prices, 3);
        assert!(result.is_some());
        let val = result.unwrap();
        // EMA should be weighted toward recent prices
        assert!(val > 30.0); // More than simple average
        assert!(val < 50.0); // Less than max
    }

    #[test]
    fn test_ema_insufficient_data() {
        let prices = vec![10.0, 20.0];
        assert_eq!(ema(&prices, 5), None);
    }

    #[test]
    fn test_rsi_uptrend() {
        // Steadily increasing prices should give high RSI
        let prices: Vec<f64> = (0..20).map(|i| 100.0 + i as f64).collect();
        let result = rsi(&prices, 14);
        assert!(result.is_some());
        assert!(result.unwrap() > 70.0, "RSI in uptrend should be > 70, got {}", result.unwrap());
    }

    #[test]
    fn test_rsi_downtrend() {
        // Steadily decreasing prices should give low RSI
        let prices: Vec<f64> = (0..20).map(|i| 100.0 - i as f64).collect();
        let result = rsi(&prices, 14);
        assert!(result.is_some());
        assert!(result.unwrap() < 30.0, "RSI in downtrend should be < 30, got {}", result.unwrap());
    }

    #[test]
    fn test_rsi_insufficient_data() {
        let prices = vec![10.0, 20.0, 30.0];
        assert_eq!(rsi(&prices, 14), None);
    }

    #[test]
    fn test_rsi_all_gains() {
        // All prices going up, no losses → RSI = 100
        let prices: Vec<f64> = (0..16).map(|i| 10.0 + i as f64 * 5.0).collect();
        let result = rsi(&prices, 14);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), 100.0);
    }

    #[test]
    fn test_rsi_signal_values() {
        assert!(rsi_signal(85.0).contains("Overbought"));
        assert!(rsi_signal(50.0).contains("Neutral"));
        assert!(rsi_signal(15.0).contains("Oversold"));
    }

    #[test]
    fn test_sma_signal_bullish() {
        let signal = sma_signal(110.0, 105.0, 100.0);
        assert!(signal.contains("Bullish"));
    }

    #[test]
    fn test_sma_signal_bearish() {
        let signal = sma_signal(90.0, 95.0, 100.0);
        assert!(signal.contains("Bearish"));
    }

    #[test]
    fn test_macd_uptrend() {
        // Steadily increasing prices — MACD should be positive
        let prices: Vec<f64> = (0..50).map(|i| 100.0 + i as f64 * 2.0).collect();
        let result = macd(&prices, 12, 26, 9);
        assert!(result.is_some(), "MACD should compute with 50 data points");
        let r = result.unwrap();
        assert!(r.macd_line > 0.0, "MACD line should be positive in uptrend, got {}", r.macd_line);
        assert!(r.histogram > 0.0 || r.histogram.abs() < 1.0, "Histogram should be positive or near zero in steady uptrend");
    }

    #[test]
    fn test_macd_downtrend() {
        // Steadily decreasing prices — MACD should be negative
        let prices: Vec<f64> = (0..50).map(|i| 200.0 - i as f64 * 2.0).collect();
        let result = macd(&prices, 12, 26, 9);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.macd_line < 0.0, "MACD line should be negative in downtrend, got {}", r.macd_line);
    }

    #[test]
    fn test_macd_insufficient_data() {
        let prices = vec![10.0, 20.0, 30.0];
        assert!(macd(&prices, 12, 26, 9).is_none());
    }

    #[test]
    fn test_macd_invalid_params() {
        let prices: Vec<f64> = (0..50).map(|i| i as f64).collect();
        // fast >= slow should return None
        assert!(macd(&prices, 26, 12, 9).is_none());
        // zero params
        assert!(macd(&prices, 0, 26, 9).is_none());
    }

    #[test]
    fn test_macd_signal_interpretation() {
        // Bullish: MACD above signal, positive
        let bullish = MacdResult { macd_line: 5.0, signal_line: 3.0, histogram: 2.0 };
        assert!(macd_signal(&bullish).contains("Bullish"));

        // Bearish: MACD below signal, negative
        let bearish = MacdResult { macd_line: -5.0, signal_line: -3.0, histogram: -2.0 };
        assert!(macd_signal(&bearish).contains("Bearish"));
    }

    #[test]
    fn test_macd_components_consistent() {
        let prices: Vec<f64> = (0..50).map(|i| 100.0 + (i as f64 * 0.5).sin() * 10.0).collect();
        let result = macd(&prices, 12, 26, 9);
        assert!(result.is_some());
        let r = result.unwrap();
        // histogram = macd_line - signal_line
        let expected_hist = r.macd_line - r.signal_line;
        assert!((r.histogram - expected_hist).abs() < 0.0001, "Histogram should equal MACD - Signal");
    }
}
