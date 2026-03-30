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
}
