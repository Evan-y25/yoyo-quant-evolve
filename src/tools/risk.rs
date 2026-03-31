//! Risk scoring — rate trades and positions by risk level.
//!
//! Provides a 1-10 risk score based on multiple factors:
//! - Position size relative to portfolio (concentration risk)
//! - Stop-loss distance (risk per trade)
//! - Indicator alignment (trend quality)
//! - Volatility context
//!
//! Risk scores help traders make more disciplined decisions.

use super::indicators;

/// Risk assessment for a single trade or position.
#[derive(Debug, Clone)]
pub struct RiskAssessment {
    /// Overall risk score (1 = very low risk, 10 = very high risk)
    pub score: u8,
    /// Risk level label
    pub level: &'static str,
    /// Individual risk factors with scores
    pub factors: Vec<(String, u8, String)>, // (factor_name, score, explanation)
    /// Summary recommendation
    pub recommendation: String,
}

impl RiskAssessment {
    /// Format risk assessment for display.
    pub fn format(&self) -> String {
        let mut output = String::new();
        let emoji = risk_emoji(self.score);

        output.push_str(&format!(
            "  ⚖️  Risk Score: {}/10 {} {}\n",
            self.score, emoji, self.level
        ));

        for (name, score, explanation) in &self.factors {
            output.push_str(&format!(
                "    {} {}: {}/10 — {}\n",
                risk_emoji(*score),
                name,
                score,
                explanation,
            ));
        }

        output.push_str(&format!("  💡 {}\n", self.recommendation));
        output
    }
}

/// Get risk emoji based on score.
fn risk_emoji(score: u8) -> &'static str {
    match score {
        0..=3 => "🟢",
        4..=6 => "🟡",
        7..=8 => "🟠",
        9..=10 => "🔴",
        _ => "⚪",
    }
}

/// Assess risk for a proposed trade.
///
/// Parameters:
/// - `portfolio_value`: total portfolio value (cash + positions)
/// - `trade_value`: notional value of the proposed trade
/// - `entry_price`: proposed entry price
/// - `stop_loss`: optional stop-loss price
/// - `prices`: historical price data for the asset (optional, for indicator analysis)
///
/// Returns a RiskAssessment with score and detailed breakdown.
pub fn assess_trade_risk(
    portfolio_value: f64,
    trade_value: f64,
    entry_price: f64,
    stop_loss: Option<f64>,
    prices: Option<&[f64]>,
) -> RiskAssessment {
    let mut factors: Vec<(String, u8, String)> = Vec::new();
    let mut total_score: f64 = 0.0;
    let mut factor_count: f64 = 0.0;

    // Factor 1: Position Size (concentration risk)
    let concentration_pct = if portfolio_value > 0.0 {
        (trade_value / portfolio_value) * 100.0
    } else {
        100.0
    };
    let size_score = match concentration_pct as u32 {
        0..=5 => 1,
        6..=10 => 2,
        11..=15 => 3,
        16..=20 => 4,
        21..=25 => 5,
        26..=35 => 6,
        36..=50 => 7,
        51..=75 => 8,
        76..=90 => 9,
        _ => 10,
    };
    factors.push((
        "Position Size".into(),
        size_score,
        format!("{:.1}% of portfolio", concentration_pct),
    ));
    total_score += size_score as f64;
    factor_count += 1.0;

    // Factor 2: Stop-Loss Risk (risk per trade as % of entry)
    if let Some(sl) = stop_loss {
        let sl_distance_pct = ((entry_price - sl).abs() / entry_price) * 100.0;
        let sl_score = match sl_distance_pct as u32 {
            0..=1 => 1, // Very tight SL
            2..=3 => 2,
            4..=5 => 3,
            6..=8 => 4,
            9..=10 => 5,
            11..=15 => 6,
            16..=20 => 7,
            21..=30 => 8,
            31..=50 => 9,
            _ => 10,
        };
        factors.push((
            "Stop-Loss".into(),
            sl_score,
            format!("{:.1}% from entry", sl_distance_pct),
        ));
        total_score += sl_score as f64;
        factor_count += 1.0;
    } else {
        // No stop-loss = higher risk
        factors.push((
            "Stop-Loss".into(),
            8,
            "No stop-loss set — unbounded risk".into(),
        ));
        total_score += 8.0;
        factor_count += 1.0;
    }

    // Factor 3: Technical Alignment (if price data available)
    if let Some(prices) = prices {
        if prices.len() >= 30 {
            let mut indicator_score = 5u8; // Neutral by default

            // Check RSI
            if let Some(rsi_val) = indicators::rsi(prices, 14) {
                if (30.0..=70.0).contains(&rsi_val) {
                    indicator_score = indicator_score.saturating_sub(1); // Safer range
                } else if rsi_val > 80.0 || rsi_val < 20.0 {
                    indicator_score = indicator_score.saturating_add(2); // Extreme = risky
                }
            }

            // Check if price is near SMA (trending cleanly = lower risk)
            if let (Some(sma7), Some(sma20)) =
                (indicators::sma(prices, 7), indicators::sma(prices, 20))
            {
                let current = *prices.last().unwrap();
                // Clear trend alignment = lower risk
                if (current > sma7 && sma7 > sma20) || (current < sma7 && sma7 < sma20) {
                    indicator_score = indicator_score.saturating_sub(1);
                } else {
                    indicator_score = indicator_score.saturating_add(1); // Mixed = riskier
                }
            }

            // Check Bollinger Band position
            if let Some(bb) = indicators::bollinger_bands(prices, 20, 2.0) {
                if bb.percent_b > 1.0 || bb.percent_b < 0.0 {
                    indicator_score = indicator_score.saturating_add(1); // Outside bands = volatile
                }
                if bb.bandwidth > 10.0 {
                    indicator_score = indicator_score.saturating_add(1); // Wide bands = high volatility
                }
            }

            indicator_score = indicator_score.min(10);

            factors.push((
                "Indicators".into(),
                indicator_score,
                if indicator_score <= 3 {
                    "Strong trend alignment".into()
                } else if indicator_score <= 6 {
                    "Mixed signals".into()
                } else {
                    "Weak or conflicting signals".into()
                },
            ));
            total_score += indicator_score as f64;
            factor_count += 1.0;
        }
    }

    // Calculate overall score (weighted average, rounded)
    let overall = if factor_count > 0.0 {
        (total_score / factor_count).round() as u8
    } else {
        5
    };
    let overall = overall.max(1).min(10);

    let level = match overall {
        1..=2 => "Low Risk",
        3..=4 => "Moderate Risk",
        5..=6 => "Elevated Risk",
        7..=8 => "High Risk",
        9..=10 => "Very High Risk",
        _ => "Unknown",
    };

    let recommendation = match overall {
        1..=3 => "Position sizing looks reasonable. Consider proceeding with your plan.".into(),
        4..=5 => "Risk is moderate. Make sure you have a clear exit plan.".into(),
        6..=7 => {
            "Risk is elevated. Consider reducing position size or tightening stop-loss.".into()
        }
        8..=9 => {
            "High risk trade. Strongly consider reducing size or waiting for better setup.".into()
        }
        10 => {
            "Extreme risk. This trade could significantly impact your portfolio. Reconsider.".into()
        }
        _ => "Unable to fully assess risk.".into(),
    };

    RiskAssessment {
        score: overall,
        level,
        factors,
        recommendation,
    }
}

/// Result of a position sizing calculation.
#[derive(Debug, Clone)]
pub struct PositionSizing {
    /// Symbol being sized
    pub symbol: String,
    /// Entry price
    pub entry_price: f64,
    /// Stop-loss price
    pub stop_loss: f64,
    /// Risk per share/unit (distance from entry to stop)
    pub risk_per_unit: f64,
    /// Risk per share as percentage of entry
    pub risk_per_unit_pct: f64,
    /// Portfolio value used for calculation
    pub portfolio_value: f64,
    /// Risk percentage of portfolio
    pub risk_pct: f64,
    /// Dollar amount at risk
    pub risk_amount: f64,
    /// Calculated position size (number of units)
    pub quantity: f64,
    /// Notional value of the position
    pub notional_value: f64,
    /// Position as percentage of portfolio
    pub position_pct: f64,
    /// Potential reward (None if no take-profit)
    pub reward_amount: Option<f64>,
    /// Risk/reward ratio (None if no take-profit)
    pub risk_reward: Option<f64>,
}

impl PositionSizing {
    /// Format position sizing result for display.
    pub fn format(&self) -> String {
        use crate::tools::format::{format_currency, format_currency_unsigned, format_price};
        let mut output = String::new();

        output.push_str(&format!("  📐 Position Sizing: {}\n", self.symbol));
        output.push_str("  ─────────────────────────────────────────\n");
        output.push_str(&format!(
            "  Entry Price:     {}\n",
            format_price(self.entry_price)
        ));
        output.push_str(&format!(
            "  Stop-Loss:       {} ({:.2}% away)\n",
            format_price(self.stop_loss),
            self.risk_per_unit_pct
        ));
        output.push_str(&format!(
            "  Risk per Unit:   {}\n",
            format_currency_unsigned(self.risk_per_unit)
        ));
        output.push_str("  ─────────────────────────────────────────\n");
        output.push_str(&format!(
            "  Portfolio Value:  {}\n",
            format_currency_unsigned(self.portfolio_value)
        ));
        output.push_str(&format!(
            "  Risk Budget:      {:.1}% → {}\n",
            self.risk_pct,
            format_currency_unsigned(self.risk_amount)
        ));
        output.push_str(&format!(
            "  ✅ Position Size:  {:.6} units\n",
            self.quantity
        ));
        output.push_str(&format!(
            "  💰 Notional Value: {} ({:.1}% of portfolio)\n",
            format_currency_unsigned(self.notional_value),
            self.position_pct
        ));

        if let (Some(reward), Some(rr)) = (self.reward_amount, self.risk_reward) {
            output.push_str("  ─────────────────────────────────────────\n");
            output.push_str(&format!(
                "  🎯 Potential Reward: {}\n",
                format_currency(reward)
            ));
            output.push_str(&format!("  📊 Risk/Reward:      1:{:.2}\n", rr));
            if rr >= 2.0 {
                output.push_str("  ✅ Good R:R ratio (≥ 1:2).\n");
            } else if rr >= 1.0 {
                output.push_str("  🟡 Marginal R:R. Consider a wider target.\n");
            } else {
                output.push_str("  🔴 Poor R:R. Risk exceeds potential reward.\n");
            }
        }

        output.push_str("  ─────────────────────────────────────────\n");
        output.push_str("  ⚠️  Not financial advice. Always verify before trading.\n");
        output
    }
}

/// Calculate optimal position size based on risk budget.
///
/// The classic position sizing formula:
///   Position Size = Risk Amount / Risk Per Unit
/// Where:
///   Risk Amount = Portfolio Value × Risk Percentage
///   Risk Per Unit = |Entry Price - Stop-Loss Price|
///
/// Parameters:
/// - `portfolio_value`: total portfolio value
/// - `entry_price`: planned entry price
/// - `stop_loss`: planned stop-loss price
/// - `risk_pct`: what percentage of portfolio to risk (e.g., 2.0 = 2%)
/// - `take_profit`: optional take-profit price (for R:R calculation)
/// - `symbol`: asset symbol (for display)
pub fn calculate_position_size(
    portfolio_value: f64,
    entry_price: f64,
    stop_loss: f64,
    risk_pct: f64,
    take_profit: Option<f64>,
    symbol: &str,
) -> Result<PositionSizing, String> {
    if portfolio_value <= 0.0 {
        return Err("Portfolio value must be positive".into());
    }
    if entry_price <= 0.0 {
        return Err("Entry price must be positive".into());
    }
    if stop_loss <= 0.0 {
        return Err("Stop-loss must be positive".into());
    }
    if risk_pct <= 0.0 || risk_pct > 100.0 {
        return Err("Risk percentage must be between 0 and 100".into());
    }
    if entry_price == stop_loss {
        return Err("Entry price and stop-loss cannot be the same".into());
    }

    let risk_per_unit = (entry_price - stop_loss).abs();
    let risk_per_unit_pct = (risk_per_unit / entry_price) * 100.0;
    let risk_amount = portfolio_value * (risk_pct / 100.0);
    let quantity = risk_amount / risk_per_unit;
    let notional_value = quantity * entry_price;
    let position_pct = (notional_value / portfolio_value) * 100.0;

    // Calculate reward if take-profit provided
    let (reward_amount, risk_reward) = if let Some(tp) = take_profit {
        let reward_per_unit = (tp - entry_price).abs();
        let total_reward = quantity * reward_per_unit;
        let rr = if risk_per_unit > 0.0 {
            reward_per_unit / risk_per_unit
        } else {
            0.0
        };
        (Some(total_reward), Some(rr))
    } else {
        (None, None)
    };

    Ok(PositionSizing {
        symbol: symbol.to_string(),
        entry_price,
        stop_loss,
        risk_per_unit,
        risk_per_unit_pct,
        portfolio_value,
        risk_pct,
        risk_amount,
        quantity,
        notional_value,
        position_pct,
        reward_amount,
        risk_reward,
    })
}

/// Suggest stop-loss levels for a given entry price using historical price data.
///
/// Uses multiple approaches:
/// 1. Percentage-based (fixed % below entry)
/// 2. Volatility-based (using close-only pseudo-ATR)
/// 3. Support/resistance level based
///
/// Returns a formatted string with suggestions, sorted by distance from entry.
pub fn suggest_stop_loss_levels(
    entry_price: f64,
    prices: &[f64],
    side: &str, // "buy" or "sell"
) -> String {
    use super::indicators;

    let mut output = String::new();
    output.push_str("  💡 Suggested Stop-Loss Levels:\n");

    let mut suggestions: Vec<(String, f64, String)> = Vec::new();

    // 1. Percentage-based stops
    for pct in [2.0, 5.0, 10.0] {
        let sl = if side == "buy" {
            entry_price * (1.0 - pct / 100.0)
        } else {
            entry_price * (1.0 + pct / 100.0)
        };
        suggestions.push((
            format!("{:.0}% stop", pct),
            sl,
            format!("{:.1}% from entry", pct),
        ));
    }

    // 2. Volatility-based (using close-only pseudo-ATR)
    if prices.len() >= 15 {
        let changes: Vec<f64> = prices.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
        let avg_change = changes.iter().sum::<f64>() / changes.len() as f64;

        for multiplier in [1.5, 2.0, 3.0] {
            let distance = avg_change * multiplier;
            let sl = if side == "buy" {
                entry_price - distance
            } else {
                entry_price + distance
            };
            let pct_from_entry = (distance / entry_price) * 100.0;
            suggestions.push((
                format!("{:.1}x ATR", multiplier),
                sl,
                format!("{:.2}% from entry", pct_from_entry),
            ));
        }
    }

    // 3. Support/resistance-based
    if prices.len() >= 20 {
        if let Some((supports, resistances)) =
            indicators::support_resistance(prices, prices.len().min(60))
        {
            if side == "buy" {
                let mut below: Vec<f64> = supports
                    .into_iter()
                    .filter(|&s| s < entry_price * 0.99)
                    .collect();
                below.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
                for (i, support) in below.iter().take(2).enumerate() {
                    let pct = ((entry_price - support) / entry_price) * 100.0;
                    suggestions.push((
                        format!("Support #{}", i + 1),
                        *support,
                        format!("{:.2}% below entry", pct),
                    ));
                }
            } else {
                let mut above: Vec<f64> = resistances
                    .into_iter()
                    .filter(|&r| r > entry_price * 1.01)
                    .collect();
                above.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                for (i, resistance) in above.iter().take(2).enumerate() {
                    let pct = ((resistance - entry_price) / entry_price) * 100.0;
                    suggestions.push((
                        format!("Resistance #{}", i + 1),
                        *resistance,
                        format!("{:.2}% above entry", pct),
                    ));
                }
            }
        }
    }

    // Sort by distance from entry (closest first)
    suggestions.sort_by(|a, b| {
        let dist_a = (a.1 - entry_price).abs();
        let dist_b = (b.1 - entry_price).abs();
        dist_a
            .partial_cmp(&dist_b)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    for (method, price, description) in suggestions.iter().take(6) {
        output.push_str(&format!(
            "     {:<14} ${:.2}  ({})\n",
            method, price, description
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_position_sizing_basic() {
        // Portfolio $100k, risk 2%, entry $100, SL $95
        // Risk amount = $2000, Risk per unit = $5, Quantity = 400
        let result = calculate_position_size(100_000.0, 100.0, 95.0, 2.0, None, "AAPL").unwrap();
        assert!((result.quantity - 400.0).abs() < 0.01);
        assert!((result.risk_amount - 2_000.0).abs() < 0.01);
        assert!((result.notional_value - 40_000.0).abs() < 0.01);
    }

    #[test]
    fn test_position_sizing_with_take_profit() {
        // Entry $100, SL $95, TP $110
        // Risk per unit = $5, Reward per unit = $10, R:R = 2.0
        let result =
            calculate_position_size(100_000.0, 100.0, 95.0, 2.0, Some(110.0), "AAPL").unwrap();
        assert!((result.risk_reward.unwrap() - 2.0).abs() < 0.01);
        assert!(result.reward_amount.is_some());
    }

    #[test]
    fn test_position_sizing_btc() {
        // Portfolio $100k, risk 1%, entry $87000, SL $85000
        // Risk amount = $1000, Risk per unit = $2000, Quantity = 0.5
        let result =
            calculate_position_size(100_000.0, 87_000.0, 85_000.0, 1.0, None, "bitcoin").unwrap();
        assert!((result.quantity - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_position_sizing_format() {
        let result =
            calculate_position_size(100_000.0, 100.0, 95.0, 2.0, Some(110.0), "AAPL").unwrap();
        let formatted = result.format();
        assert!(formatted.contains("Position Sizing"));
        assert!(formatted.contains("AAPL"));
        assert!(formatted.contains("Risk Budget"));
        assert!(formatted.contains("Risk/Reward"));
    }

    #[test]
    fn test_position_sizing_invalid_inputs() {
        assert!(calculate_position_size(0.0, 100.0, 95.0, 2.0, None, "X").is_err());
        assert!(calculate_position_size(100_000.0, 0.0, 95.0, 2.0, None, "X").is_err());
        assert!(calculate_position_size(100_000.0, 100.0, 100.0, 2.0, None, "X").is_err());
        assert!(calculate_position_size(100_000.0, 100.0, 95.0, 0.0, None, "X").is_err());
        assert!(calculate_position_size(100_000.0, 100.0, 95.0, 101.0, None, "X").is_err());
    }

    #[test]
    fn test_position_sizing_short() {
        // Short: entry $100, SL $105 (above), risk per unit = $5
        let result = calculate_position_size(100_000.0, 100.0, 105.0, 2.0, None, "AAPL").unwrap();
        assert!((result.quantity - 400.0).abs() < 0.01);
        assert!((result.risk_per_unit - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_small_position_low_risk() {
        let assessment = assess_trade_risk(100_000.0, 5_000.0, 100.0, Some(95.0), None);
        assert!(
            assessment.score <= 4,
            "Small position with SL should be low risk, got {}",
            assessment.score
        );
    }

    #[test]
    fn test_large_position_high_risk() {
        let assessment = assess_trade_risk(100_000.0, 80_000.0, 100.0, None, None);
        assert!(
            assessment.score >= 7,
            "Large position without SL should be high risk, got {}",
            assessment.score
        );
    }

    #[test]
    fn test_no_stop_loss_increases_risk() {
        let with_sl = assess_trade_risk(100_000.0, 10_000.0, 100.0, Some(95.0), None);
        let without_sl = assess_trade_risk(100_000.0, 10_000.0, 100.0, None, None);
        assert!(
            without_sl.score >= with_sl.score,
            "No SL should be equal or higher risk: {} vs {}",
            without_sl.score,
            with_sl.score
        );
    }

    #[test]
    fn test_risk_score_bounds() {
        // Very small trade
        let assessment = assess_trade_risk(1_000_000.0, 1_000.0, 100.0, Some(99.0), None);
        assert!(assessment.score >= 1 && assessment.score <= 10);

        // Very large trade
        let assessment = assess_trade_risk(100.0, 100.0, 100.0, None, None);
        assert!(assessment.score >= 1 && assessment.score <= 10);
    }

    #[test]
    fn test_risk_with_indicator_data() {
        // Uptrending price data
        let prices: Vec<f64> = (0..50).map(|i| 100.0 + i as f64 * 0.5).collect();
        let assessment = assess_trade_risk(100_000.0, 10_000.0, 125.0, Some(120.0), Some(&prices));
        assert!(
            assessment.factors.len() == 3,
            "Should have 3 factors with indicator data"
        );
        assert!(assessment.score >= 1 && assessment.score <= 10);
    }

    #[test]
    fn test_risk_assessment_format() {
        let assessment = assess_trade_risk(100_000.0, 10_000.0, 100.0, Some(95.0), None);
        let formatted = assessment.format();
        assert!(formatted.contains("Risk Score"));
        assert!(formatted.contains("Position Size"));
        assert!(formatted.contains("Stop-Loss"));
    }

    #[test]
    fn test_risk_level_labels() {
        let low = assess_trade_risk(1_000_000.0, 10_000.0, 100.0, Some(99.0), None);
        assert!(low.level.contains("Low") || low.level.contains("Moderate"));

        let high = assess_trade_risk(100_000.0, 90_000.0, 100.0, None, None);
        assert!(high.level.contains("High") || high.level.contains("Elevated"));
    }

    #[test]
    fn test_tight_stop_loss_lower_risk() {
        let tight = assess_trade_risk(100_000.0, 10_000.0, 100.0, Some(99.0), None); // 1% SL
        let wide = assess_trade_risk(100_000.0, 10_000.0, 100.0, Some(80.0), None); // 20% SL
        assert!(
            tight.score <= wide.score,
            "Tight SL should be lower or equal risk: {} vs {}",
            tight.score,
            wide.score
        );
    }

    #[test]
    fn test_suggest_stop_loss_buy() {
        // Uptrending prices
        let prices: Vec<f64> = (0..60)
            .map(|i| 100.0 + i as f64 * 0.5 + (i as f64 * 0.3).sin() * 2.0)
            .collect();
        let current = *prices.last().unwrap();
        let result = suggest_stop_loss_levels(current, &prices, "buy");
        assert!(result.contains("Suggested Stop-Loss Levels"));
        assert!(result.contains("2% stop"));
        assert!(result.contains("ATR"));
    }

    #[test]
    fn test_suggest_stop_loss_sell() {
        let prices: Vec<f64> = (0..60)
            .map(|i| 200.0 - i as f64 * 0.3 + (i as f64 * 0.2).sin() * 3.0)
            .collect();
        let current = *prices.last().unwrap();
        let result = suggest_stop_loss_levels(current, &prices, "sell");
        assert!(result.contains("Suggested Stop-Loss Levels"));
        assert!(result.contains("2% stop"));
    }

    #[test]
    fn test_suggest_stop_loss_short_data() {
        let prices = vec![100.0, 101.0, 99.0, 102.0, 100.5];
        let result = suggest_stop_loss_levels(100.5, &prices, "buy");
        // Should still have percentage-based stops even with minimal data
        assert!(result.contains("2% stop"));
    }
}
