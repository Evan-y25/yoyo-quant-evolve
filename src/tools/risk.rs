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
            0..=1 => 1,  // Very tight SL
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
        6..=7 => "Risk is elevated. Consider reducing position size or tightening stop-loss.".into(),
        8..=9 => "High risk trade. Strongly consider reducing size or waiting for better setup."
            .into(),
        10 => "Extreme risk. This trade could significantly impact your portfolio. Reconsider."
            .into(),
        _ => "Unable to fully assess risk.".into(),
    };

    RiskAssessment {
        score: overall,
        level,
        factors,
        recommendation,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_small_position_low_risk() {
        let assessment = assess_trade_risk(100_000.0, 5_000.0, 100.0, Some(95.0), None);
        assert!(assessment.score <= 4, "Small position with SL should be low risk, got {}", assessment.score);
    }

    #[test]
    fn test_large_position_high_risk() {
        let assessment = assess_trade_risk(100_000.0, 80_000.0, 100.0, None, None);
        assert!(assessment.score >= 7, "Large position without SL should be high risk, got {}", assessment.score);
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
        assert!(assessment.factors.len() == 3, "Should have 3 factors with indicator data");
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
        let wide = assess_trade_risk(100_000.0, 10_000.0, 100.0, Some(80.0), None);  // 20% SL
        assert!(
            tight.score <= wide.score,
            "Tight SL should be lower or equal risk: {} vs {}",
            tight.score,
            wide.score
        );
    }
}
