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
///
/// Returns a value between 0-100.
/// - Above 70: overbought
/// - Below 30: oversold
///
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

/// Bollinger Bands result.
#[derive(Debug, Clone)]
pub struct BollingerBands {
    /// Middle band (SMA)
    pub middle: f64,
    /// Upper band (SMA + k * stddev)
    pub upper: f64,
    /// Lower band (SMA - k * stddev)
    pub lower: f64,
    /// Bandwidth: (upper - lower) / middle * 100 — measures volatility
    pub bandwidth: f64,
    /// %B: (price - lower) / (upper - lower) — position within bands
    /// > 1.0 means price above upper band, < 0.0 means below lower band
    pub percent_b: f64,
}

/// Calculate Bollinger Bands.
///
/// Standard parameters: period=20, k=2.0 (standard deviations).
/// Returns None if there aren't enough data points.
///
/// Bollinger Bands measure volatility and relative price levels:
/// - Price near upper band → potentially overbought
/// - Price near lower band → potentially oversold
/// - Bands narrowing (squeeze) → low volatility, breakout possible
/// - Bands widening → high volatility, trend in progress
pub fn bollinger_bands(prices: &[f64], period: usize, k: f64) -> Option<BollingerBands> {
    if prices.len() < period || period == 0 {
        return None;
    }

    let window = &prices[prices.len() - period..];
    let middle = window.iter().sum::<f64>() / period as f64;

    // Standard deviation
    let variance = window.iter().map(|&p| (p - middle).powi(2)).sum::<f64>() / period as f64;
    let stddev = variance.sqrt();

    let upper = middle + k * stddev;
    let lower = middle - k * stddev;

    let bandwidth = if middle > 0.0 {
        (upper - lower) / middle * 100.0
    } else {
        0.0
    };

    let current_price = *prices.last().unwrap();
    let band_width = upper - lower;
    let percent_b = if band_width > 0.0 {
        (current_price - lower) / band_width
    } else {
        0.5 // Default to middle if bands have zero width
    };

    Some(BollingerBands {
        middle,
        upper,
        lower,
        bandwidth,
        percent_b,
    })
}

/// Interpret Bollinger Bands position as a human-readable signal.
pub fn bollinger_signal(bb: &BollingerBands) -> &'static str {
    if bb.percent_b > 1.0 {
        "🔴 Above upper band (potentially overbought)"
    } else if bb.percent_b > 0.8 {
        "🟠 Near upper band (watch for reversal)"
    } else if bb.percent_b > 0.5 {
        "🟢 Upper half (bullish positioning)"
    } else if bb.percent_b > 0.2 {
        "🟡 Lower half (bearish positioning)"
    } else if bb.percent_b >= 0.0 {
        "🟠 Near lower band (watch for bounce)"
    } else {
        "🟢 Below lower band (potentially oversold)"
    }
}

/// Calculate Volume Weighted Average Price (VWAP).
///
/// VWAP = Σ(price × volume) / Σ(volume)
///
/// This is a key benchmark for institutional traders:
/// - Price above VWAP → buyers are in control
/// - Price below VWAP → sellers are in control
/// - Often acts as support/resistance level
///
/// Returns None if no valid volume data or all volumes are zero.
pub fn vwap(prices: &[f64], volumes: &[f64]) -> Option<f64> {
    if prices.is_empty() || volumes.is_empty() || prices.len() != volumes.len() {
        return None;
    }

    let total_pv: f64 = prices.iter().zip(volumes.iter()).map(|(p, v)| p * v).sum();
    let total_vol: f64 = volumes.iter().sum();

    if total_vol <= 0.0 {
        return None;
    }

    Some(total_pv / total_vol)
}

/// Calculate Average True Range (ATR) for a given period.
///
/// ATR measures volatility by looking at the range of price movement.
/// It uses High, Low, Close data. Standard period is 14.
///
/// True Range = max of:
///   - Current High - Current Low
///   - |Current High - Previous Close|
///   - |Current Low - Previous Close|
///
/// ATR = Smoothed average of True Range values.
///
/// Returns None if there aren't enough data points.
pub fn atr(highs: &[f64], lows: &[f64], closes: &[f64], period: usize) -> Option<f64> {
    if highs.len() < period + 1
        || lows.len() < period + 1
        || closes.len() < period + 1
        || period == 0
        || highs.len() != lows.len()
        || highs.len() != closes.len()
    {
        return None;
    }

    // Calculate True Range for each bar (starting from index 1)
    let mut true_ranges = Vec::with_capacity(highs.len() - 1);
    for i in 1..highs.len() {
        let hl = highs[i] - lows[i];
        let hc = (highs[i] - closes[i - 1]).abs();
        let lc = (lows[i] - closes[i - 1]).abs();
        true_ranges.push(hl.max(hc).max(lc));
    }

    if true_ranges.len() < period {
        return None;
    }

    // Initial ATR = average of first `period` true ranges
    let mut atr_val: f64 = true_ranges[..period].iter().sum::<f64>() / period as f64;

    // Smooth using Wilder's method
    for &tr in &true_ranges[period..] {
        atr_val = (atr_val * (period as f64 - 1.0) + tr) / period as f64;
    }

    Some(atr_val)
}

/// Interpret ATR as a volatility signal relative to price.
pub fn atr_signal(atr_value: f64, current_price: f64) -> &'static str {
    if current_price <= 0.0 {
        return "⚪ Unknown";
    }
    let atr_pct = (atr_value / current_price) * 100.0;
    if atr_pct > 5.0 {
        "🔴 Very High Volatility (ATR > 5% of price)"
    } else if atr_pct > 3.0 {
        "🟠 High Volatility (ATR 3-5% of price)"
    } else if atr_pct > 1.5 {
        "🟡 Moderate Volatility (ATR 1.5-3% of price)"
    } else if atr_pct > 0.5 {
        "🟢 Low Volatility (ATR 0.5-1.5% of price)"
    } else {
        "⚪ Very Low Volatility (ATR < 0.5% of price)"
    }
}

/// Calculate simple Support and Resistance levels from price data.
///
/// Uses recent highs/lows to identify key levels:
/// - Resistance: recent high, and the highest recent swing
/// - Support: recent low, and the lowest recent swing
///
/// Returns (support_levels, resistance_levels) sorted.
pub fn support_resistance(prices: &[f64], period: usize) -> Option<(Vec<f64>, Vec<f64>)> {
    if prices.len() < period || period < 5 {
        return None;
    }

    let window = &prices[prices.len() - period..];
    let current = *prices.last().unwrap();

    // Find local minima and maxima (simple swing detection)
    let mut supports = Vec::new();
    let mut resistances = Vec::new();

    // Use the absolute high and low
    let period_high = window.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    let period_low = window.iter().copied().fold(f64::INFINITY, f64::min);

    if period_high > current {
        resistances.push(period_high);
    }
    if period_low < current {
        supports.push(period_low);
    }

    // Find swing highs/lows (points higher/lower than 2 neighbors)
    for i in 2..window.len() - 2 {
        let p = window[i];
        if p > window[i - 1] && p > window[i - 2] && p > window[i + 1] && p > window[i + 2] {
            // Swing high — potential resistance
            if p > current {
                resistances.push(p);
            }
        }
        if p < window[i - 1] && p < window[i - 2] && p < window[i + 1] && p < window[i + 2] {
            // Swing low — potential support
            if p < current {
                supports.push(p);
            }
        }
    }

    // Deduplicate similar levels (within 0.5% of each other)
    supports = dedup_levels(supports);
    resistances = dedup_levels(resistances);

    // Sort: supports descending (closest to price first), resistances ascending
    supports.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    resistances.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    // Keep top 3 each
    supports.truncate(3);
    resistances.truncate(3);

    Some((supports, resistances))
}

/// Merge levels that are within 0.5% of each other (take the average).
fn dedup_levels(mut levels: Vec<f64>) -> Vec<f64> {
    if levels.is_empty() {
        return levels;
    }
    levels.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));

    let mut merged = vec![levels[0]];
    for &level in &levels[1..] {
        let last = merged.last().unwrap();
        if (level - last) / last < 0.005 {
            // Merge: average them
            let avg = (*last + level) / 2.0;
            *merged.last_mut().unwrap() = avg;
        } else {
            merged.push(level);
        }
    }
    merged
}

/// Interpret VWAP position as a human-readable signal.
pub fn vwap_signal(current_price: f64, vwap_value: f64) -> &'static str {
    let diff_pct = ((current_price - vwap_value) / vwap_value) * 100.0;
    if diff_pct > 3.0 {
        "🟢 Well above VWAP (strong buyer control)"
    } else if diff_pct > 0.5 {
        "🟢 Above VWAP (buyers favored)"
    } else if diff_pct > -0.5 {
        "⚪ Near VWAP (equilibrium)"
    } else if diff_pct > -3.0 {
        "🔴 Below VWAP (sellers favored)"
    } else {
        "🔴 Well below VWAP (strong seller control)"
    }
}

/// Stochastic Oscillator result.
#[derive(Debug, Clone)]
pub struct StochasticResult {
    /// %K line: (Current Close - Lowest Low) / (Highest High - Lowest Low) * 100
    pub k: f64,
    /// %D line: SMA of %K over signal_period
    pub d: f64,
}

/// Calculate Stochastic Oscillator.
///
/// Standard parameters: period=14, signal_period=3.
/// Uses high/low/close data.
///
/// The Stochastic Oscillator measures momentum:
/// - %K above 80: overbought
/// - %K below 20: oversold
/// - %K crossing above %D: bullish signal
/// - %K crossing below %D: bearish signal
///
/// Returns None if there aren't enough data points.
pub fn stochastic(
    highs: &[f64],
    lows: &[f64],
    closes: &[f64],
    period: usize,
    signal_period: usize,
) -> Option<StochasticResult> {
    if highs.len() < period
        || lows.len() < period
        || closes.len() < period
        || period == 0
        || signal_period == 0
        || highs.len() != lows.len()
        || highs.len() != closes.len()
    {
        return None;
    }

    let len = closes.len();

    // We need enough %K values to calculate %D
    if len < period + signal_period - 1 {
        return None;
    }

    // Calculate %K for each valid window
    let mut k_values = Vec::with_capacity(len - period + 1);
    for i in (period - 1)..len {
        let window_start = i + 1 - period;
        let highest_high = highs[window_start..=i]
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        let lowest_low = lows[window_start..=i]
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let range = highest_high - lowest_low;
        let k = if range > 0.0 {
            ((closes[i] - lowest_low) / range) * 100.0
        } else {
            50.0 // Default to middle if no range
        };
        k_values.push(k);
    }

    if k_values.len() < signal_period {
        return None;
    }

    // Current %K is the last value
    let current_k = *k_values.last().unwrap();

    // %D = SMA of last signal_period %K values
    let d_window = &k_values[k_values.len() - signal_period..];
    let current_d = d_window.iter().sum::<f64>() / signal_period as f64;

    Some(StochasticResult {
        k: current_k,
        d: current_d,
    })
}

/// Interpret Stochastic Oscillator as a human-readable signal.
pub fn stochastic_signal(result: &StochasticResult) -> &'static str {
    if result.k >= 80.0 && result.k > result.d {
        "🔴 Overbought (%K > 80, above %D)"
    } else if result.k >= 80.0 && result.k <= result.d {
        "🟠 Overbought, losing momentum (%K > 80, crossing below %D)"
    } else if result.k <= 20.0 && result.k < result.d {
        "🟢 Oversold (%K < 20, below %D)"
    } else if result.k <= 20.0 && result.k >= result.d {
        "🟠 Oversold, gaining momentum (%K < 20, crossing above %D)"
    } else if result.k > result.d {
        "🟢 Bullish (%K above %D)"
    } else if result.k < result.d {
        "🔴 Bearish (%K below %D)"
    } else {
        "⚪ Neutral"
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

    #[test]
    fn test_bollinger_bands_basic() {
        // Flat prices → narrow bands
        let prices = vec![100.0; 20];
        let result = bollinger_bands(&prices, 20, 2.0);
        assert!(result.is_some());
        let bb = result.unwrap();
        assert_eq!(bb.middle, 100.0);
        assert_eq!(bb.upper, 100.0); // Zero stddev → bands collapse
        assert_eq!(bb.lower, 100.0);
        assert_eq!(bb.bandwidth, 0.0);
    }

    #[test]
    fn test_bollinger_bands_volatile() {
        // Alternating prices should create wider bands
        let mut prices = Vec::new();
        for i in 0..20 {
            if i % 2 == 0 { prices.push(110.0); } else { prices.push(90.0); }
        }
        let result = bollinger_bands(&prices, 20, 2.0);
        assert!(result.is_some());
        let bb = result.unwrap();
        assert_eq!(bb.middle, 100.0); // Average of 110 and 90
        assert!(bb.upper > 100.0, "Upper band should be above middle");
        assert!(bb.lower < 100.0, "Lower band should be below middle");
        assert!(bb.bandwidth > 0.0, "Bandwidth should be positive for volatile prices");
    }

    #[test]
    fn test_bollinger_bands_insufficient_data() {
        let prices = vec![10.0, 20.0];
        assert!(bollinger_bands(&prices, 20, 2.0).is_none());
    }

    #[test]
    fn test_bollinger_percent_b() {
        // Price at upper band → %B ≈ 1.0
        // Price at lower band → %B ≈ 0.0
        // Price at middle → %B ≈ 0.5
        let mut prices = Vec::new();
        for i in 0..19 {
            if i % 2 == 0 { prices.push(110.0); } else { prices.push(90.0); }
        }
        // Last price at the middle
        prices.push(100.0);
        let result = bollinger_bands(&prices, 20, 2.0);
        assert!(result.is_some());
        let bb = result.unwrap();
        assert!((bb.percent_b - 0.5).abs() < 0.1, "%B should be near 0.5 when price is at middle, got {}", bb.percent_b);
    }

    #[test]
    fn test_bollinger_signal_interpretation() {
        let overbought = BollingerBands { middle: 100.0, upper: 120.0, lower: 80.0, bandwidth: 40.0, percent_b: 1.1 };
        assert!(bollinger_signal(&overbought).contains("overbought"));

        let oversold = BollingerBands { middle: 100.0, upper: 120.0, lower: 80.0, bandwidth: 40.0, percent_b: -0.1 };
        assert!(bollinger_signal(&oversold).contains("oversold"));

        let middle = BollingerBands { middle: 100.0, upper: 120.0, lower: 80.0, bandwidth: 40.0, percent_b: 0.6 };
        assert!(bollinger_signal(&middle).contains("bullish"));
    }

    #[test]
    fn test_vwap_basic() {
        let prices = vec![100.0, 102.0, 98.0, 101.0, 103.0];
        let volumes = vec![1000.0, 2000.0, 1500.0, 1000.0, 3000.0];
        let result = vwap(&prices, &volumes);
        assert!(result.is_some());
        let v = result.unwrap();
        // Manual: (100*1000 + 102*2000 + 98*1500 + 101*1000 + 103*3000) / (1000+2000+1500+1000+3000)
        // = (100000 + 204000 + 147000 + 101000 + 309000) / 8500
        // = 861000 / 8500 = 101.29...
        assert!((v - 101.294).abs() < 0.01, "VWAP should be ~101.29, got {}", v);
    }

    #[test]
    fn test_vwap_empty() {
        assert!(vwap(&[], &[]).is_none());
    }

    #[test]
    fn test_vwap_mismatched_lengths() {
        assert!(vwap(&[100.0], &[1000.0, 2000.0]).is_none());
    }

    #[test]
    fn test_vwap_zero_volume() {
        assert!(vwap(&[100.0, 200.0], &[0.0, 0.0]).is_none());
    }

    #[test]
    fn test_vwap_signal() {
        assert!(vwap_signal(105.0, 100.0).contains("above VWAP"));
        assert!(vwap_signal(95.0, 100.0).contains("below VWAP"));
        assert!(vwap_signal(100.0, 100.0).contains("Near VWAP"));
    }

    #[test]
    fn test_atr_basic() {
        // Simple case: steady prices with known ranges
        let highs =  vec![12.0, 12.5, 13.0, 12.8, 13.2, 12.9, 13.1, 12.7, 13.3, 13.5, 13.0, 13.2, 13.4, 13.1, 13.6, 13.3];
        let lows =   vec![10.0, 10.5, 11.0, 10.8, 11.2, 10.9, 11.1, 10.7, 11.3, 11.5, 11.0, 11.2, 11.4, 11.1, 11.6, 11.3];
        let closes = vec![11.0, 11.5, 12.0, 11.8, 12.2, 11.9, 12.1, 11.7, 12.3, 12.5, 12.0, 12.2, 12.4, 12.1, 12.6, 12.3];
        let result = atr(&highs, &lows, &closes, 14);
        assert!(result.is_some(), "ATR should compute with 16 data points and period 14");
        let val = result.unwrap();
        assert!(val > 0.0, "ATR should be positive, got {}", val);
    }

    #[test]
    fn test_atr_insufficient_data() {
        let highs = vec![12.0, 12.5];
        let lows = vec![10.0, 10.5];
        let closes = vec![11.0, 11.5];
        assert!(atr(&highs, &lows, &closes, 14).is_none());
    }

    #[test]
    fn test_atr_mismatched_lengths() {
        let highs = vec![12.0, 12.5, 13.0];
        let lows = vec![10.0, 10.5];
        let closes = vec![11.0, 11.5, 12.0];
        assert!(atr(&highs, &lows, &closes, 2).is_none());
    }

    #[test]
    fn test_atr_signal() {
        // 6% ATR on a $100 stock = very high
        assert!(atr_signal(6.0, 100.0).contains("Very High"));
        // 1% ATR = low
        assert!(atr_signal(1.0, 100.0).contains("Low Volatility"));
        // 0.3% = very low
        assert!(atr_signal(0.3, 100.0).contains("Very Low"));
    }

    #[test]
    fn test_support_resistance_basic() {
        // Create a price series with clear swing highs and lows
        let mut prices = Vec::new();
        // Upswing
        for i in 0..10 { prices.push(100.0 + i as f64); }
        // Downswing
        for i in 0..10 { prices.push(109.0 - i as f64); }
        // Upswing again
        for i in 0..10 { prices.push(100.0 + i as f64 * 0.8); }
        // Price ends at ~107

        let result = support_resistance(&prices, 30);
        assert!(result.is_some(), "Should detect support/resistance levels");
        let (supports, resistances) = result.unwrap();
        // Should find some levels
        assert!(!supports.is_empty() || !resistances.is_empty(), "Should find at least one level");
    }

    #[test]
    fn test_support_resistance_insufficient_data() {
        let prices = vec![100.0, 101.0, 102.0];
        assert!(support_resistance(&prices, 20).is_none());
    }

    #[test]
    fn test_dedup_levels() {
        // Levels within 0.5% of each other should merge
        let levels = vec![100.0, 100.3, 105.0, 105.4, 110.0];
        let result = dedup_levels(levels);
        // 100.0 and 100.3 are 0.3% apart → merge
        // 105.0 and 105.4 are ~0.38% → merge
        assert!(result.len() <= 3, "Should merge close levels, got {:?}", result);
    }

    #[test]
    fn test_stochastic_uptrend() {
        // Steadily increasing prices — %K should be high
        let highs: Vec<f64> = (0..20).map(|i| 102.0 + i as f64).collect();
        let lows: Vec<f64> = (0..20).map(|i| 98.0 + i as f64).collect();
        let closes: Vec<f64> = (0..20).map(|i| 101.0 + i as f64).collect();
        let result = stochastic(&highs, &lows, &closes, 14, 3);
        assert!(result.is_some(), "Stochastic should compute with 20 data points");
        let r = result.unwrap();
        assert!(r.k > 50.0, "%K should be high in uptrend, got {}", r.k);
    }

    #[test]
    fn test_stochastic_downtrend() {
        // Steadily decreasing prices — %K should be low
        let highs: Vec<f64> = (0..20).map(|i| 122.0 - i as f64).collect();
        let lows: Vec<f64> = (0..20).map(|i| 118.0 - i as f64).collect();
        let closes: Vec<f64> = (0..20).map(|i| 119.0 - i as f64).collect();
        let result = stochastic(&highs, &lows, &closes, 14, 3);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.k < 50.0, "%K should be low in downtrend, got {}", r.k);
    }

    #[test]
    fn test_stochastic_insufficient_data() {
        let highs = vec![12.0, 12.5];
        let lows = vec![10.0, 10.5];
        let closes = vec![11.0, 11.5];
        assert!(stochastic(&highs, &lows, &closes, 14, 3).is_none());
    }

    #[test]
    fn test_stochastic_mismatched_lengths() {
        let highs = vec![12.0; 20];
        let lows = vec![10.0; 19];
        let closes = vec![11.0; 20];
        assert!(stochastic(&highs, &lows, &closes, 14, 3).is_none());
    }

    #[test]
    fn test_stochastic_flat_prices() {
        // All same prices → %K should be 50 (default)
        let highs = vec![100.0; 20];
        let lows = vec![100.0; 20];
        let closes = vec![100.0; 20];
        let result = stochastic(&highs, &lows, &closes, 14, 3);
        assert!(result.is_some());
        let r = result.unwrap();
        assert_eq!(r.k, 50.0, "%K should be 50 for flat prices, got {}", r.k);
    }

    #[test]
    fn test_stochastic_signal_overbought() {
        let overbought = StochasticResult { k: 85.0, d: 80.0 };
        assert!(stochastic_signal(&overbought).contains("Overbought"));
    }

    #[test]
    fn test_stochastic_signal_oversold() {
        let oversold = StochasticResult { k: 15.0, d: 18.0 };
        assert!(stochastic_signal(&oversold).contains("Oversold"));
    }

    #[test]
    fn test_stochastic_signal_bullish() {
        let bullish = StochasticResult { k: 55.0, d: 45.0 };
        assert!(stochastic_signal(&bullish).contains("Bullish"));
    }

    #[test]
    fn test_stochastic_signal_bearish() {
        let bearish = StochasticResult { k: 45.0, d: 55.0 };
        assert!(stochastic_signal(&bearish).contains("Bearish"));
    }

    #[test]
    fn test_stochastic_range_bounds() {
        // %K should always be between 0 and 100
        let mut highs = Vec::new();
        let mut lows = Vec::new();
        let mut closes = Vec::new();
        for i in 0..20 {
            highs.push(100.0 + (i as f64 * 0.7).sin() * 10.0 + 5.0);
            lows.push(100.0 + (i as f64 * 0.7).sin() * 10.0 - 5.0);
            closes.push(100.0 + (i as f64 * 0.7).sin() * 10.0);
        }
        let result = stochastic(&highs, &lows, &closes, 14, 3);
        assert!(result.is_some());
        let r = result.unwrap();
        assert!(r.k >= 0.0 && r.k <= 100.0, "%K should be 0-100, got {}", r.k);
        assert!(r.d >= 0.0 && r.d <= 100.0, "%D should be 0-100, got {}", r.d);
    }
}
