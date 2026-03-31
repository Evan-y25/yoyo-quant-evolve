//! Trade analysis — detect recurring patterns in trading behavior.
//!
//! This module analyzes closed trades from the portfolio to find:
//! - Recurring mistake patterns (e.g., always losing on a specific symbol)
//! - Confidence calibration (are high-confidence trades actually better?)
//! - Holding losers too long / cutting winners short
//! - Loss streaks and their impact on subsequent decisions
//!
//! This is the "learning from mistakes" engine that closes the feedback loop.

use super::portfolio::{PaperTrade, Portfolio};

/// A detected pattern in trading behavior.
#[derive(Debug, Clone)]
pub struct TradingPattern {
    /// Pattern category
    pub category: PatternCategory,
    /// Human-readable description
    pub description: String,
    /// Severity: 1 (minor) to 5 (critical)
    pub severity: u8,
    /// Actionable suggestion
    pub suggestion: String,
    /// Trade IDs that exhibit this pattern
    pub trade_ids: Vec<u32>,
}

/// Categories of trading patterns we detect.
#[derive(Debug, Clone, PartialEq)]
pub enum PatternCategory {
    SymbolBias,
    OverConfidence,
    UnderConfidence,
    HoldingLosers,
    CuttingWinners,
    LossStreak,
    NoStopLoss,
    Concentration,
}

impl TradingPattern {
    pub fn format(&self) -> String {
        let severity_emoji = match self.severity {
            1 => "💡",
            2 => "⚠️ ",
            3 => "🟡",
            4 => "🟠",
            5 => "🔴",
            _ => "⚪",
        };
        let category_label = match self.category {
            PatternCategory::SymbolBias => "Symbol Bias",
            PatternCategory::OverConfidence => "Overconfidence",
            PatternCategory::UnderConfidence => "Underconfidence",
            PatternCategory::HoldingLosers => "Holding Losers",
            PatternCategory::CuttingWinners => "Cutting Winners",
            PatternCategory::LossStreak => "Loss Streak",
            PatternCategory::NoStopLoss => "No Stop-Loss",
            PatternCategory::Concentration => "Concentration",
        };
        format!(
            "  {} [{}] {}\n    → {}",
            severity_emoji, category_label, self.description, self.suggestion,
        )
    }
}

/// Result of analyzing trading patterns.
pub struct TradeAnalysisReport {
    pub patterns: Vec<TradingPattern>,
    pub total_trades_analyzed: usize,
    pub overall_health: &'static str,
    pub health_score: u8,
}

impl TradeAnalysisReport {
    pub fn format(&self) -> String {
        let mut output = String::new();
        let health_emoji = match self.health_score {
            8..=10 => "🟢",
            5..=7 => "🟡",
            3..=4 => "🟠",
            _ => "🔴",
        };

        output.push_str("🔍 Trade Pattern Analysis\n");
        output.push_str("═════════════════════════════════════════\n");
        output.push_str(&format!(
            "  Trades Analyzed: {}\n",
            self.total_trades_analyzed
        ));
        output.push_str(&format!(
            "  Trading Health:  {}/10 {} {}\n",
            self.health_score, health_emoji, self.overall_health
        ));

        if self.patterns.is_empty() {
            output.push_str("─────────────────────────────────────────\n");
            output.push_str("  ✅ No concerning patterns detected. Keep it up!\n");
            output.push_str("  💡 More trades = more data = better pattern detection.\n");
        } else {
            output.push_str("─────────────────────────────────────────\n");
            output.push_str(&format!(
                "  ⚠️  {} pattern(s) detected:\n",
                self.patterns.len()
            ));
            output.push_str("─────────────────────────────────────────\n");

            let mut sorted: Vec<&TradingPattern> = self.patterns.iter().collect();
            sorted.sort_by(|a, b| b.severity.cmp(&a.severity));

            for pattern in sorted {
                output.push_str(&pattern.format());
                output.push('\n');
            }
        }

        output.push_str("═════════════════════════════════════════\n");
        output.push_str("  💡 Review these patterns to improve your trading.\n");
        output.push_str("  ⚠️  Pattern detection improves with more trades.\n");
        output
    }
}

/// Analyze a portfolio's closed trades for recurring patterns.
pub fn analyze_trades(portfolio: &Portfolio) -> TradeAnalysisReport {
    let closed: Vec<&PaperTrade> = portfolio.trades.iter().filter(|t| !t.is_open()).collect();

    if closed.len() < 3 {
        return TradeAnalysisReport {
            patterns: Vec::new(),
            total_trades_analyzed: closed.len(),
            overall_health: "Insufficient data",
            health_score: 5,
        };
    }

    let mut patterns: Vec<TradingPattern> = Vec::new();

    detect_symbol_bias(&closed, &mut patterns);
    detect_confidence_issues(&closed, &mut patterns);
    detect_holding_losers(&closed, &mut patterns);
    detect_cutting_winners(&closed, &mut patterns);
    detect_loss_streaks(&closed, &mut patterns);
    detect_no_stop_loss(&closed, &mut patterns);
    detect_concentration(&closed, &mut patterns);

    let total_severity: u32 = patterns.iter().map(|p| p.severity as u32).sum();
    let health_score = if patterns.is_empty() {
        9u8
    } else {
        let deduction = (total_severity * 2).min(8);
        (10u8).saturating_sub(deduction as u8).max(1)
    };

    let overall_health = match health_score {
        9..=10 => "Excellent — disciplined trading",
        7..=8 => "Good — minor issues to watch",
        5..=6 => "Fair — some patterns need attention",
        3..=4 => "Concerning — review your approach",
        _ => "Critical — significant issues detected",
    };

    TradeAnalysisReport {
        patterns,
        total_trades_analyzed: closed.len(),
        overall_health,
        health_score,
    }
}

fn detect_symbol_bias(trades: &[&PaperTrade], patterns: &mut Vec<TradingPattern>) {
    let mut symbol_stats: std::collections::HashMap<&str, (u32, u32, Vec<u32>)> =
        std::collections::HashMap::new();

    for trade in trades {
        let pnl = trade.realized_pnl.unwrap_or(0.0);
        let entry = symbol_stats
            .entry(&trade.symbol)
            .or_insert((0, 0, Vec::new()));
        if pnl > 0.0 {
            entry.0 += 1;
        } else {
            entry.1 += 1;
        }
        entry.2.push(trade.id);
    }

    for (symbol, (wins, losses, ids)) in &symbol_stats {
        let total = wins + losses;
        if total >= 3 && *losses > *wins && *losses >= 3 {
            let loss_rate = *losses as f64 / total as f64 * 100.0;
            patterns.push(TradingPattern {
                category: PatternCategory::SymbolBias,
                description: format!(
                    "Losing {:.0}% of trades on {} ({} wins, {} losses)",
                    loss_rate, symbol, wins, losses
                ),
                severity: if loss_rate >= 80.0 { 4 } else { 3 },
                suggestion: format!(
                    "Consider avoiding {} or adjusting your strategy for this asset.",
                    symbol
                ),
                trade_ids: ids.clone(),
            });
        }
    }
}

fn detect_confidence_issues(trades: &[&PaperTrade], patterns: &mut Vec<TradingPattern>) {
    let high_conf: Vec<&PaperTrade> = trades
        .iter()
        .filter(|t| t.confidence >= 7)
        .copied()
        .collect();
    let low_conf: Vec<&PaperTrade> = trades
        .iter()
        .filter(|t| t.confidence <= 4)
        .copied()
        .collect();

    if high_conf.len() >= 3 {
        let wins = high_conf
            .iter()
            .filter(|t| t.realized_pnl.unwrap_or(0.0) > 0.0)
            .count();
        let wr = (wins as f64 / high_conf.len() as f64) * 100.0;
        if wr < 40.0 {
            patterns.push(TradingPattern {
                category: PatternCategory::OverConfidence,
                description: format!(
                    "High confidence (7-10) trades have only {:.0}% win rate ({} of {})",
                    wr,
                    wins,
                    high_conf.len()
                ),
                severity: 4,
                suggestion:
                    "Your confidence doesn't match outcomes. Be more critical of 'sure thing' trades."
                        .into(),
                trade_ids: high_conf.iter().map(|t| t.id).collect(),
            });
        }
    }

    if low_conf.len() >= 3 {
        let wins = low_conf
            .iter()
            .filter(|t| t.realized_pnl.unwrap_or(0.0) > 0.0)
            .count();
        let wr = (wins as f64 / low_conf.len() as f64) * 100.0;
        if wr > 70.0 {
            patterns.push(TradingPattern {
                category: PatternCategory::UnderConfidence,
                description: format!(
                    "Low confidence (1-4) trades have {:.0}% win rate ({} of {})!",
                    wr,
                    wins,
                    low_conf.len()
                ),
                severity: 2,
                suggestion:
                    "You're better than you think! Consider sizing up on these 'unsure' trades."
                        .into(),
                trade_ids: low_conf.iter().map(|t| t.id).collect(),
            });
        }
    }
}

fn detect_holding_losers(trades: &[&PaperTrade], patterns: &mut Vec<TradingPattern>) {
    let big_losers: Vec<&PaperTrade> = trades
        .iter()
        .filter(|t| {
            let pnl = t.realized_pnl.unwrap_or(0.0);
            let notional = t.notional_value();
            let pnl_pct = if notional > 0.0 {
                (pnl / notional) * 100.0
            } else {
                0.0
            };
            pnl_pct < -10.0
        })
        .copied()
        .collect();

    if big_losers.len() >= 2 {
        let avg_loss_pct: f64 = big_losers
            .iter()
            .map(|t| {
                let pnl = t.realized_pnl.unwrap_or(0.0);
                let notional = t.notional_value();
                if notional > 0.0 {
                    (pnl / notional) * 100.0
                } else {
                    0.0
                }
            })
            .sum::<f64>()
            / big_losers.len() as f64;

        patterns.push(TradingPattern {
            category: PatternCategory::HoldingLosers,
            description: format!(
                "{} trades lost more than 10% each (avg loss: {:.1}%)",
                big_losers.len(),
                avg_loss_pct
            ),
            severity: if avg_loss_pct < -20.0 { 5 } else { 3 },
            suggestion: "Use tighter stop-losses. Consider the 2% rule: never risk more than 2% of portfolio per trade.".into(),
            trade_ids: big_losers.iter().map(|t| t.id).collect(),
        });
    }
}

fn detect_cutting_winners(trades: &[&PaperTrade], patterns: &mut Vec<TradingPattern>) {
    let winners: Vec<&PaperTrade> = trades
        .iter()
        .filter(|t| t.realized_pnl.unwrap_or(0.0) > 0.0)
        .copied()
        .collect();
    let losers: Vec<&PaperTrade> = trades
        .iter()
        .filter(|t| t.realized_pnl.unwrap_or(0.0) < 0.0)
        .copied()
        .collect();

    if winners.len() >= 3 && losers.len() >= 3 {
        let avg_win_pct: f64 = winners
            .iter()
            .map(|t| {
                let pnl = t.realized_pnl.unwrap_or(0.0);
                let notional = t.notional_value();
                if notional > 0.0 {
                    (pnl / notional) * 100.0
                } else {
                    0.0
                }
            })
            .sum::<f64>()
            / winners.len() as f64;

        let avg_loss_pct: f64 = losers
            .iter()
            .map(|t| {
                let pnl = t.realized_pnl.unwrap_or(0.0);
                let notional = t.notional_value();
                if notional > 0.0 {
                    (pnl / notional).abs() * 100.0
                } else {
                    0.0
                }
            })
            .sum::<f64>()
            / losers.len() as f64;

        if avg_win_pct < avg_loss_pct * 0.75 {
            patterns.push(TradingPattern {
                category: PatternCategory::CuttingWinners,
                description: format!(
                    "Average win ({:.1}%) is smaller than average loss ({:.1}%)",
                    avg_win_pct, avg_loss_pct
                ),
                severity: 3,
                suggestion: "Let winners run longer! Use trailing stops instead of fixed targets."
                    .into(),
                trade_ids: winners.iter().map(|t| t.id).collect(),
            });
        }
    }
}

fn detect_loss_streaks(trades: &[&PaperTrade], patterns: &mut Vec<TradingPattern>) {
    let mut current_streak = 0u32;
    let mut streak_ids: Vec<u32> = Vec::new();
    let mut worst_streak = 0u32;
    let mut worst_streak_ids: Vec<u32> = Vec::new();

    for trade in trades {
        let pnl = trade.realized_pnl.unwrap_or(0.0);
        if pnl <= 0.0 {
            current_streak += 1;
            streak_ids.push(trade.id);
            if current_streak > worst_streak {
                worst_streak = current_streak;
                worst_streak_ids = streak_ids.clone();
            }
        } else {
            current_streak = 0;
            streak_ids.clear();
        }
    }

    if worst_streak >= 3 {
        patterns.push(TradingPattern {
            category: PatternCategory::LossStreak,
            description: format!("{} consecutive losing trades detected", worst_streak),
            severity: if worst_streak >= 5 { 4 } else { 2 },
            suggestion: "After 3+ losses, consider pausing to reset. Tilt leads to bigger losses."
                .into(),
            trade_ids: worst_streak_ids,
        });
    }
}

fn detect_no_stop_loss(trades: &[&PaperTrade], patterns: &mut Vec<TradingPattern>) {
    let losses_without_sl: Vec<&PaperTrade> = trades
        .iter()
        .filter(|t| t.realized_pnl.unwrap_or(0.0) < 0.0 && t.stop_loss.is_none())
        .copied()
        .collect();

    let total_losses = trades
        .iter()
        .filter(|t| t.realized_pnl.unwrap_or(0.0) < 0.0)
        .count();

    if losses_without_sl.len() >= 2 && total_losses > 0 {
        let pct = (losses_without_sl.len() as f64 / total_losses as f64) * 100.0;
        patterns.push(TradingPattern {
            category: PatternCategory::NoStopLoss,
            description: format!(
                "{} of {} losing trades ({:.0}%) had no stop-loss set",
                losses_without_sl.len(),
                total_losses,
                pct,
            ),
            severity: if pct >= 80.0 { 4 } else { 3 },
            suggestion: "Always set a stop-loss before entering a trade. Use /pf sl <id> <price>."
                .into(),
            trade_ids: losses_without_sl.iter().map(|t| t.id).collect(),
        });
    }
}

fn detect_concentration(trades: &[&PaperTrade], patterns: &mut Vec<TradingPattern>) {
    let mut symbol_count: std::collections::HashMap<&str, (u32, Vec<u32>)> =
        std::collections::HashMap::new();

    for trade in trades {
        let entry = symbol_count.entry(&trade.symbol).or_insert((0, Vec::new()));
        entry.0 += 1;
        entry.1.push(trade.id);
    }

    let total = trades.len() as f64;
    for (symbol, (count, ids)) in &symbol_count {
        let pct = *count as f64 / total * 100.0;
        if *count >= 5 && pct >= 60.0 {
            patterns.push(TradingPattern {
                category: PatternCategory::Concentration,
                description: format!(
                    "{:.0}% of trades ({} of {}) are in {}",
                    pct,
                    count,
                    trades.len(),
                    symbol
                ),
                severity: 3,
                suggestion: "Diversify across more assets to reduce concentration risk.".into(),
                trade_ids: ids.clone(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_trade(
        id: u32,
        symbol: &str,
        side: &str,
        qty: f64,
        entry: f64,
        exit: f64,
        confidence: u8,
        stop_loss: Option<f64>,
    ) -> PaperTrade {
        let pnl = if side == "buy" {
            qty * (exit - entry)
        } else {
            qty * (entry - exit)
        };
        PaperTrade {
            id,
            symbol: symbol.to_string(),
            side: side.to_string(),
            quantity: qty,
            entry_price: entry,
            exit_price: Some(exit),
            reasoning: String::new(),
            confidence,
            entry_time: "2025-01-01T00:00Z".to_string(),
            exit_time: Some("2025-01-02T00:00Z".to_string()),
            realized_pnl: Some(pnl),
            stop_loss,
            take_profit: None,
            trailing_stop_pct: None,
            highest_price_seen: None,
            lowest_price_seen: None,
        }
    }

    fn portfolio_with_trades(trades: Vec<PaperTrade>) -> Portfolio {
        Portfolio {
            starting_balance: 100_000.0,
            cash: 100_000.0,
            trades,
            next_id: 100,
        }
    }

    #[test]
    fn test_insufficient_data() {
        let portfolio = portfolio_with_trades(vec![make_trade(
            1, "AAPL", "buy", 10.0, 100.0, 110.0, 5, None,
        )]);
        let report = analyze_trades(&portfolio);
        assert_eq!(report.total_trades_analyzed, 1);
        assert!(report.patterns.is_empty());
        assert_eq!(report.overall_health, "Insufficient data");
    }

    #[test]
    fn test_symbol_bias_detection() {
        let trades = vec![
            make_trade(1, "AAPL", "buy", 10.0, 100.0, 90.0, 5, None),
            make_trade(2, "AAPL", "buy", 10.0, 95.0, 85.0, 5, None),
            make_trade(3, "AAPL", "buy", 10.0, 90.0, 80.0, 5, None),
            make_trade(4, "MSFT", "buy", 10.0, 200.0, 220.0, 5, None),
        ];
        let portfolio = portfolio_with_trades(trades);
        let report = analyze_trades(&portfolio);
        let found = report
            .patterns
            .iter()
            .any(|p| p.category == PatternCategory::SymbolBias);
        assert!(found, "Should detect AAPL symbol bias");
    }

    #[test]
    fn test_overconfidence_detection() {
        let trades = vec![
            make_trade(1, "AAPL", "buy", 10.0, 100.0, 90.0, 8, None),
            make_trade(2, "MSFT", "buy", 10.0, 200.0, 180.0, 9, None),
            make_trade(3, "TSLA", "buy", 10.0, 300.0, 270.0, 7, None),
            make_trade(4, "GOOG", "buy", 10.0, 150.0, 160.0, 3, None),
        ];
        let portfolio = portfolio_with_trades(trades);
        let report = analyze_trades(&portfolio);
        let found = report
            .patterns
            .iter()
            .any(|p| p.category == PatternCategory::OverConfidence);
        assert!(found, "Should detect overconfidence");
    }

    #[test]
    fn test_loss_streak_detection() {
        let trades = vec![
            make_trade(1, "AAPL", "buy", 10.0, 100.0, 95.0, 5, None),
            make_trade(2, "MSFT", "buy", 10.0, 200.0, 190.0, 5, None),
            make_trade(3, "TSLA", "buy", 10.0, 300.0, 280.0, 5, None),
            make_trade(4, "GOOG", "buy", 10.0, 150.0, 160.0, 5, None),
        ];
        let portfolio = portfolio_with_trades(trades);
        let report = analyze_trades(&portfolio);
        let found = report
            .patterns
            .iter()
            .any(|p| p.category == PatternCategory::LossStreak);
        assert!(found, "Should detect 3-loss streak");
    }

    #[test]
    fn test_no_stop_loss_detection() {
        let trades = vec![
            make_trade(1, "AAPL", "buy", 10.0, 100.0, 90.0, 5, None),
            make_trade(2, "MSFT", "buy", 10.0, 200.0, 180.0, 5, None),
            make_trade(3, "TSLA", "buy", 10.0, 300.0, 310.0, 5, Some(280.0)),
        ];
        let portfolio = portfolio_with_trades(trades);
        let report = analyze_trades(&portfolio);
        let found = report
            .patterns
            .iter()
            .any(|p| p.category == PatternCategory::NoStopLoss);
        assert!(found, "Should detect missing stop-losses");
    }

    #[test]
    fn test_holding_losers_detection() {
        let trades = vec![
            make_trade(1, "AAPL", "buy", 10.0, 100.0, 80.0, 5, None),
            make_trade(2, "MSFT", "buy", 10.0, 200.0, 160.0, 5, None),
            make_trade(3, "TSLA", "buy", 10.0, 300.0, 310.0, 5, None),
        ];
        let portfolio = portfolio_with_trades(trades);
        let report = analyze_trades(&portfolio);
        let found = report
            .patterns
            .iter()
            .any(|p| p.category == PatternCategory::HoldingLosers);
        assert!(found, "Should detect holding losers too long");
    }

    #[test]
    fn test_cutting_winners_detection() {
        let trades = vec![
            make_trade(1, "A", "buy", 10.0, 100.0, 101.0, 5, None),
            make_trade(2, "B", "buy", 10.0, 100.0, 101.5, 5, None),
            make_trade(3, "C", "buy", 10.0, 100.0, 100.5, 5, None),
            make_trade(4, "D", "buy", 10.0, 100.0, 90.0, 5, None),
            make_trade(5, "E", "buy", 10.0, 100.0, 88.0, 5, None),
            make_trade(6, "F", "buy", 10.0, 100.0, 85.0, 5, None),
        ];
        let portfolio = portfolio_with_trades(trades);
        let report = analyze_trades(&portfolio);
        let found = report
            .patterns
            .iter()
            .any(|p| p.category == PatternCategory::CuttingWinners);
        assert!(found, "Should detect cutting winners short");
    }

    #[test]
    fn test_healthy_portfolio() {
        let trades = vec![
            make_trade(1, "AAPL", "buy", 10.0, 100.0, 110.0, 7, Some(95.0)),
            make_trade(2, "MSFT", "buy", 10.0, 200.0, 220.0, 6, Some(190.0)),
            make_trade(3, "TSLA", "buy", 10.0, 300.0, 290.0, 5, Some(280.0)),
        ];
        let portfolio = portfolio_with_trades(trades);
        let report = analyze_trades(&portfolio);
        assert!(
            report.health_score >= 5,
            "Healthy portfolio should have decent health score, got {}",
            report.health_score
        );
    }

    #[test]
    fn test_concentration_detection() {
        let trades = vec![
            make_trade(1, "BTC", "buy", 0.1, 80000.0, 81000.0, 5, None),
            make_trade(2, "BTC", "buy", 0.1, 81000.0, 82000.0, 5, None),
            make_trade(3, "BTC", "buy", 0.1, 82000.0, 83000.0, 5, None),
            make_trade(4, "BTC", "buy", 0.1, 83000.0, 84000.0, 5, None),
            make_trade(5, "BTC", "buy", 0.1, 84000.0, 85000.0, 5, None),
            make_trade(6, "ETH", "buy", 1.0, 3000.0, 3100.0, 5, None),
        ];
        let portfolio = portfolio_with_trades(trades);
        let report = analyze_trades(&portfolio);
        let found = report
            .patterns
            .iter()
            .any(|p| p.category == PatternCategory::Concentration);
        assert!(found, "Should detect BTC concentration");
    }

    #[test]
    fn test_report_format() {
        let trades = vec![
            make_trade(1, "AAPL", "buy", 10.0, 100.0, 90.0, 8, None),
            make_trade(2, "MSFT", "buy", 10.0, 200.0, 180.0, 9, None),
            make_trade(3, "TSLA", "buy", 10.0, 300.0, 270.0, 7, None),
        ];
        let portfolio = portfolio_with_trades(trades);
        let report = analyze_trades(&portfolio);
        let formatted = report.format();
        assert!(formatted.contains("Trade Pattern Analysis"));
        assert!(formatted.contains("Trading Health:"));
        assert!(formatted.contains("Trades Analyzed: 3"));
    }

    #[test]
    fn test_empty_portfolio() {
        let portfolio = portfolio_with_trades(vec![]);
        let report = analyze_trades(&portfolio);
        assert_eq!(report.total_trades_analyzed, 0);
        assert!(report.patterns.is_empty());
    }

    #[test]
    fn test_underconfidence_detection() {
        let trades = vec![
            make_trade(1, "AAPL", "buy", 10.0, 100.0, 120.0, 2, None),
            make_trade(2, "MSFT", "buy", 10.0, 200.0, 230.0, 3, None),
            make_trade(3, "TSLA", "buy", 10.0, 300.0, 340.0, 4, None),
            make_trade(4, "GOOG", "buy", 10.0, 150.0, 140.0, 8, None),
        ];
        let portfolio = portfolio_with_trades(trades);
        let report = analyze_trades(&portfolio);
        let found = report
            .patterns
            .iter()
            .any(|p| p.category == PatternCategory::UnderConfidence);
        assert!(found, "Should detect underconfidence");
    }
}
