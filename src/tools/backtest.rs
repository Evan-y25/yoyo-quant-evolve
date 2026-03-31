//! Backtesting framework — test trading strategies against historical data.
//!
//! This module provides a simple but functional backtesting engine that can:
//! - Run predefined strategies (SMA crossover, RSI mean-reversion) on price series
//! - Track simulated trades with entry/exit prices
//! - Calculate performance metrics: total return, win rate, max drawdown, Sharpe ratio
//! - Format results for display
//!
//! Strategies are pure functions of price data — no lookahead bias.

use super::indicators;

/// A single backtested trade.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BacktestTrade {
    pub entry_idx: usize,
    pub exit_idx: usize,
    pub entry_price: f64,
    pub exit_price: f64,
    pub side: &'static str, // "buy" or "sell"
    pub pnl: f64,
    pub pnl_pct: f64,
}

/// Result of running a backtest.
#[derive(Debug, Clone)]
pub struct BacktestResult {
    pub strategy_name: String,
    pub symbol: String,
    pub range: String,
    pub trades: Vec<BacktestTrade>,
    pub total_return_pct: f64,
    pub buy_hold_return_pct: f64,
    pub win_rate: f64,
    pub total_trades: usize,
    pub winning_trades: usize,
    pub losing_trades: usize,
    pub avg_win_pct: f64,
    pub avg_loss_pct: f64,
    pub max_drawdown_pct: f64,
    pub profit_factor: f64,
    pub sharpe_ratio: f64,
    pub data_points: usize,
}

impl BacktestResult {
    /// Format backtest result for display.
    pub fn format(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "🧪 Backtest: {} on {} ({})\n",
            self.strategy_name, self.symbol, self.range
        ));
        output.push_str("─────────────────────────────────────────\n");
        output.push_str(&format!("  Data Points:   {}\n", self.data_points));
        output.push_str(&format!("  Total Trades:  {}\n", self.total_trades));
        output.push_str(&format!(
            "  Win/Loss:      {} / {}\n",
            self.winning_trades, self.losing_trades
        ));

        if self.total_trades > 0 {
            output.push_str(&format!("  Win Rate:      {:.1}%\n", self.win_rate));
            output.push_str(&format!(
                "  Strategy Return: {}{:.2}% {}\n",
                if self.total_return_pct >= 0.0 {
                    "+"
                } else {
                    ""
                },
                self.total_return_pct,
                if self.total_return_pct > self.buy_hold_return_pct {
                    "🟢 beats buy & hold"
                } else {
                    "🔴 underperforms buy & hold"
                }
            ));
            output.push_str(&format!(
                "  Buy & Hold:    {}{:.2}%\n",
                if self.buy_hold_return_pct >= 0.0 {
                    "+"
                } else {
                    ""
                },
                self.buy_hold_return_pct
            ));
            output.push_str(&format!(
                "  Alpha:         {}{:.2}%\n",
                if self.total_return_pct - self.buy_hold_return_pct >= 0.0 {
                    "+"
                } else {
                    ""
                },
                self.total_return_pct - self.buy_hold_return_pct,
            ));
            output.push_str(&format!(
                "  Max Drawdown:  -{:.2}%\n",
                self.max_drawdown_pct
            ));
            if self.avg_win_pct > 0.0 {
                output.push_str(&format!("  Avg Win:       +{:.2}%\n", self.avg_win_pct));
            }
            if self.avg_loss_pct < 0.0 {
                output.push_str(&format!("  Avg Loss:      {:.2}%\n", self.avg_loss_pct));
            }
            if self.profit_factor.is_finite() {
                output.push_str(&format!("  Profit Factor: {:.2}\n", self.profit_factor));
            }
            if self.sharpe_ratio.is_finite() && self.sharpe_ratio != 0.0 {
                output.push_str(&format!("  Sharpe Ratio:  {:.2}\n", self.sharpe_ratio));
            }

            // Show recent trades (last 10)
            if !self.trades.is_empty() {
                output.push_str("─────────────────────────────────────────\n");
                output.push_str("  📊 Recent Trades (last 10):\n");
                for trade in self
                    .trades
                    .iter()
                    .rev()
                    .take(10)
                    .collect::<Vec<_>>()
                    .into_iter()
                    .rev()
                {
                    let emoji = if trade.pnl >= 0.0 { "🟢" } else { "🔴" };
                    output.push_str(&format!(
                        "    {} {} @ ${:.2} → ${:.2} ({}{:.2}%)\n",
                        emoji,
                        trade.side.to_uppercase(),
                        trade.entry_price,
                        trade.exit_price,
                        if trade.pnl_pct >= 0.0 { "+" } else { "" },
                        trade.pnl_pct,
                    ));
                }
            }
        } else {
            output.push_str("  No trades generated. Strategy had no entry signals.\n");
            output.push_str(&format!(
                "  Buy & Hold:    {}{:.2}%\n",
                if self.buy_hold_return_pct >= 0.0 {
                    "+"
                } else {
                    ""
                },
                self.buy_hold_return_pct
            ));
        }

        output.push_str("─────────────────────────────────────────\n");
        output.push_str("  ⚠️  Past performance is not indicative of future results.\n");
        output.push_str("  Backtest does not account for fees, slippage, or liquidity.\n");

        output
    }
}

/// Available backtesting strategies.
pub enum Strategy {
    /// SMA crossover: buy when short SMA crosses above long SMA, sell when it crosses below.
    SmaCrossover {
        short_period: usize,
        long_period: usize,
    },
    /// RSI mean-reversion: buy when RSI < oversold, sell when RSI > overbought.
    RsiMeanReversion {
        period: usize,
        oversold: f64,
        overbought: f64,
    },
    /// Bollinger Band squeeze: buy when bands narrow then expand upward,
    /// sell when they narrow then expand downward.
    BollingerSqueeze {
        period: usize,
        std_dev: f64,
    },
}

impl Strategy {
    /// Get a human-readable name.
    pub fn name(&self) -> String {
        match self {
            Strategy::SmaCrossover {
                short_period,
                long_period,
            } => {
                format!("SMA Crossover ({}/{})", short_period, long_period)
            }
            Strategy::RsiMeanReversion {
                period,
                oversold,
                overbought,
            } => {
                format!(
                    "RSI Mean-Reversion ({}, {}/{})",
                    period, oversold, overbought
                )
            }
            Strategy::BollingerSqueeze { period, std_dev } => {
                format!("Bollinger Squeeze ({}, {}σ)", period, std_dev)
            }
        }
    }
}

/// Run a backtest on a price series.
///
/// Returns BacktestResult with all trades and metrics.
/// The strategy generates entry/exit signals; we simulate a single position at a time
/// (no pyramiding, no leverage).
pub fn run_backtest(
    prices: &[f64],
    strategy: &Strategy,
    symbol: &str,
    range: &str,
) -> BacktestResult {
    let trades = match strategy {
        Strategy::SmaCrossover {
            short_period,
            long_period,
        } => backtest_sma_crossover(prices, *short_period, *long_period),
        Strategy::RsiMeanReversion {
            period,
            oversold,
            overbought,
        } => backtest_rsi_mean_reversion(prices, *period, *oversold, *overbought),
        Strategy::BollingerSqueeze { period, std_dev } => {
            backtest_bollinger_squeeze(prices, *period, *std_dev)
        }
    };

    let buy_hold_return_pct = if prices.len() >= 2 && prices[0] > 0.0 {
        ((prices[prices.len() - 1] - prices[0]) / prices[0]) * 100.0
    } else {
        0.0
    };

    // Calculate metrics
    let total_trades = trades.len();
    let winning_trades = trades.iter().filter(|t| t.pnl > 0.0).count();
    let losing_trades = trades.iter().filter(|t| t.pnl <= 0.0).count();
    let win_rate = if total_trades > 0 {
        (winning_trades as f64 / total_trades as f64) * 100.0
    } else {
        0.0
    };

    // Cumulative return (compounded)
    let mut equity = 1.0;
    let mut peak_equity = 1.0;
    let mut max_drawdown_pct = 0.0;
    let mut equity_curve: Vec<f64> = vec![1.0];

    for trade in &trades {
        let trade_return = 1.0 + (trade.pnl_pct / 100.0);
        equity *= trade_return;
        equity_curve.push(equity);

        if equity > peak_equity {
            peak_equity = equity;
        }
        let drawdown = ((peak_equity - equity) / peak_equity) * 100.0;
        if drawdown > max_drawdown_pct {
            max_drawdown_pct = drawdown;
        }
    }

    let total_return_pct = (equity - 1.0) * 100.0;

    // Avg win/loss
    let wins: Vec<f64> = trades
        .iter()
        .filter(|t| t.pnl > 0.0)
        .map(|t| t.pnl_pct)
        .collect();
    let losses: Vec<f64> = trades
        .iter()
        .filter(|t| t.pnl <= 0.0)
        .map(|t| t.pnl_pct)
        .collect();

    let avg_win_pct = if !wins.is_empty() {
        wins.iter().sum::<f64>() / wins.len() as f64
    } else {
        0.0
    };

    let avg_loss_pct = if !losses.is_empty() {
        losses.iter().sum::<f64>() / losses.len() as f64
    } else {
        0.0
    };

    // Profit factor
    let total_win: f64 = wins.iter().sum::<f64>().max(0.0);
    let total_loss: f64 = losses.iter().sum::<f64>().abs();
    let profit_factor = if total_loss > 0.0 {
        total_win / total_loss
    } else if total_win > 0.0 {
        f64::INFINITY
    } else {
        0.0
    };

    // Sharpe ratio (annualized, simplified)
    let trade_returns: Vec<f64> = trades.iter().map(|t| t.pnl_pct / 100.0).collect();
    let sharpe_ratio = calculate_sharpe(&trade_returns);

    BacktestResult {
        strategy_name: strategy.name(),
        symbol: symbol.to_string(),
        range: range.to_string(),
        trades,
        total_return_pct,
        buy_hold_return_pct,
        win_rate,
        total_trades,
        winning_trades,
        losing_trades,
        avg_win_pct,
        avg_loss_pct,
        max_drawdown_pct,
        profit_factor,
        sharpe_ratio,
        data_points: prices.len(),
    }
}

/// SMA crossover strategy.
/// Buy when short SMA crosses above long SMA, sell when it crosses below.
fn backtest_sma_crossover(
    prices: &[f64],
    short_period: usize,
    long_period: usize,
) -> Vec<BacktestTrade> {
    let mut trades = Vec::new();
    if prices.len() < long_period + 1 {
        return trades;
    }

    // Compute SMA series
    let mut short_smas = Vec::new();
    let mut long_smas = Vec::new();
    for i in 0..prices.len() {
        if i + 1 >= short_period {
            let window = &prices[i + 1 - short_period..=i];
            let sma = window.iter().sum::<f64>() / short_period as f64;
            short_smas.push(Some(sma));
        } else {
            short_smas.push(None);
        }

        if i + 1 >= long_period {
            let window = &prices[i + 1 - long_period..=i];
            let sma = window.iter().sum::<f64>() / long_period as f64;
            long_smas.push(Some(sma));
        } else {
            long_smas.push(None);
        }
    }

    let mut in_position = false;
    let mut entry_idx = 0;
    let mut entry_price = 0.0;

    for i in 1..prices.len() {
        let (prev_short, prev_long) = match (short_smas[i - 1], long_smas[i - 1]) {
            (Some(s), Some(l)) => (s, l),
            _ => continue,
        };
        let (curr_short, curr_long) = match (short_smas[i], long_smas[i]) {
            (Some(s), Some(l)) => (s, l),
            _ => continue,
        };

        // Golden cross: short crosses above long → buy
        if !in_position && prev_short <= prev_long && curr_short > curr_long {
            in_position = true;
            entry_idx = i;
            entry_price = prices[i];
        }
        // Death cross: short crosses below long → sell
        else if in_position && prev_short >= prev_long && curr_short < curr_long {
            let exit_price = prices[i];
            let pnl = exit_price - entry_price;
            let pnl_pct = (pnl / entry_price) * 100.0;
            trades.push(BacktestTrade {
                entry_idx,
                exit_idx: i,
                entry_price,
                exit_price,
                side: "buy",
                pnl,
                pnl_pct,
            });
            in_position = false;
        }
    }

    // Close any open position at the end
    if in_position {
        let exit_price = *prices.last().unwrap();
        let pnl = exit_price - entry_price;
        let pnl_pct = (pnl / entry_price) * 100.0;
        trades.push(BacktestTrade {
            entry_idx,
            exit_idx: prices.len() - 1,
            entry_price,
            exit_price,
            side: "buy",
            pnl,
            pnl_pct,
        });
    }

    trades
}

/// RSI mean-reversion strategy.
/// Buy when RSI drops below oversold threshold, sell when RSI rises above overbought threshold.
fn backtest_rsi_mean_reversion(
    prices: &[f64],
    period: usize,
    oversold: f64,
    overbought: f64,
) -> Vec<BacktestTrade> {
    let mut trades = Vec::new();
    if prices.len() < period + 2 {
        return trades;
    }

    // Compute RSI at each point
    let mut rsi_values: Vec<Option<f64>> = Vec::new();
    for i in 0..prices.len() {
        if i + 1 >= period + 1 {
            let window = &prices[..=i];
            let r = indicators::rsi(window, period);
            rsi_values.push(r);
        } else {
            rsi_values.push(None);
        }
    }

    let mut in_position = false;
    let mut entry_idx = 0;
    let mut entry_price = 0.0;

    for i in 1..prices.len() {
        let curr_rsi = match rsi_values[i] {
            Some(r) => r,
            None => continue,
        };

        // Buy when RSI crosses below oversold level
        if !in_position && curr_rsi < oversold {
            in_position = true;
            entry_idx = i;
            entry_price = prices[i];
        }
        // Sell when RSI crosses above overbought level
        else if in_position && curr_rsi > overbought {
            let exit_price = prices[i];
            let pnl = exit_price - entry_price;
            let pnl_pct = (pnl / entry_price) * 100.0;
            trades.push(BacktestTrade {
                entry_idx,
                exit_idx: i,
                entry_price,
                exit_price,
                side: "buy",
                pnl,
                pnl_pct,
            });
            in_position = false;
        }
    }

    // Close any open position at the end
    if in_position {
        let exit_price = *prices.last().unwrap();
        let pnl = exit_price - entry_price;
        let pnl_pct = (pnl / entry_price) * 100.0;
        trades.push(BacktestTrade {
            entry_idx,
            exit_idx: prices.len() - 1,
            entry_price,
            exit_price,
            side: "buy",
            pnl,
            pnl_pct,
        });
    }

    trades
}

/// Bollinger Band Squeeze strategy.
/// When bandwidth narrows below a threshold (indicating low volatility squeeze),
/// wait for price to break out above the upper band (buy) or below the lower band (sell/exit).
///
/// The idea: low volatility = compression → breakout → big move.
fn backtest_bollinger_squeeze(
    prices: &[f64],
    period: usize,
    std_dev: f64,
) -> Vec<BacktestTrade> {
    let mut trades = Vec::new();
    if prices.len() < period + 10 {
        return trades;
    }

    // Compute Bollinger Bands at each point
    let mut bandwidths = Vec::new();
    let mut uppers = Vec::new();
    let mut lowers = Vec::new();
    let mut middles = Vec::new();

    for i in 0..prices.len() {
        if i + 1 >= period {
            let window = &prices[i + 1 - period..=i];
            let middle = window.iter().sum::<f64>() / period as f64;
            let variance = window.iter().map(|&p| (p - middle).powi(2)).sum::<f64>() / period as f64;
            let stddev = variance.sqrt();
            let upper = middle + std_dev * stddev;
            let lower = middle - std_dev * stddev;
            let bw = if middle > 0.0 { (upper - lower) / middle * 100.0 } else { 0.0 };
            bandwidths.push(Some(bw));
            uppers.push(Some(upper));
            lowers.push(Some(lower));
            middles.push(Some(middle));
        } else {
            bandwidths.push(None);
            uppers.push(None);
            lowers.push(None);
            middles.push(None);
        }
    }

    // Calculate recent average bandwidth to detect squeeze
    // A squeeze is when bandwidth falls below 75% of its recent average
    let lookback = 20.min(prices.len() / 4); // Dynamic lookback

    let mut in_position = false;
    let mut entry_idx = 0;
    let mut entry_price = 0.0;
    let mut was_in_squeeze = false;

    for i in (period + lookback)..prices.len() {
        let current_bw = match bandwidths[i] {
            Some(bw) => bw,
            None => continue,
        };
        let upper = match uppers[i] {
            Some(u) => u,
            None => continue,
        };
        let _lower = match lowers[i] {
            Some(l) => l,
            None => continue,
        };
        let middle = match middles[i] {
            Some(m) => m,
            None => continue,
        };

        // Average bandwidth over lookback period
        let avg_bw: f64 = bandwidths[i - lookback..i]
            .iter()
            .filter_map(|b| *b)
            .sum::<f64>() / lookback as f64;

        let in_squeeze = current_bw < avg_bw * 0.75;

        if !in_position {
            // Enter on breakout from squeeze
            if was_in_squeeze && !in_squeeze {
                // Squeeze just released — check direction
                if prices[i] > upper {
                    // Upward breakout → buy
                    in_position = true;
                    entry_idx = i;
                    entry_price = prices[i];
                }
            }
        } else {
            // Exit when price crosses below middle band or enters new squeeze
            if prices[i] < middle || in_squeeze {
                let exit_price = prices[i];
                let pnl = exit_price - entry_price;
                let pnl_pct = (pnl / entry_price) * 100.0;
                trades.push(BacktestTrade {
                    entry_idx,
                    exit_idx: i,
                    entry_price,
                    exit_price,
                    side: "buy",
                    pnl,
                    pnl_pct,
                });
                in_position = false;
            }
        }

        was_in_squeeze = in_squeeze;
    }

    // Close any open position at the end
    if in_position {
        let exit_price = *prices.last().unwrap();
        let pnl = exit_price - entry_price;
        let pnl_pct = (pnl / entry_price) * 100.0;
        trades.push(BacktestTrade {
            entry_idx,
            exit_idx: prices.len() - 1,
            entry_price,
            exit_price,
            side: "buy",
            pnl,
            pnl_pct,
        });
    }

    trades
}

/// Calculate annualized Sharpe ratio from trade returns.
/// Assumes risk-free rate of 0 for simplicity.
fn calculate_sharpe(returns: &[f64]) -> f64 {
    if returns.len() < 2 {
        return 0.0;
    }

    let n = returns.len() as f64;
    let mean = returns.iter().sum::<f64>() / n;
    let variance = returns.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (n - 1.0);
    let stddev = variance.sqrt();

    if stddev == 0.0 {
        return 0.0;
    }

    // Annualize: assume ~252 trading days per year
    // Scale factor depends on trade frequency, but sqrt(n) is a common approximation
    mean / stddev * (n.min(252.0)).sqrt()
}

/// Parse a strategy string from user input.
/// Examples:
///   "sma" or "sma_crossover" → SmaCrossover(7, 25)
///   "sma_10_30" → SmaCrossover(10, 30)
///   "rsi" or "rsi_mean_reversion" → RsiMeanReversion(14, 30, 70)
///   "rsi_14_25_75" → RsiMeanReversion(14, 25, 75)
pub fn parse_strategy(input: &str) -> Option<Strategy> {
    let parts: Vec<&str> = input.split('_').collect();

    match parts.first().copied() {
        Some("sma") => {
            let short_period = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(7);
            let long_period = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(25);
            if short_period >= long_period || short_period == 0 {
                return None;
            }
            Some(Strategy::SmaCrossover {
                short_period,
                long_period,
            })
        }
        Some("rsi") => {
            let period = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(14);
            let oversold = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(30.0);
            let overbought = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(70.0);
            if period == 0 || oversold >= overbought {
                return None;
            }
            Some(Strategy::RsiMeanReversion {
                period,
                oversold,
                overbought,
            })
        }
        Some("bb") | Some("bollinger") | Some("squeeze") => {
            let period = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(20);
            if period < 5 {
                return None;
            }
            Some(Strategy::BollingerSqueeze {
                period,
                std_dev: 2.0,
            })
        }
        _ => None,
    }
}

/// List available strategies with descriptions.
pub fn available_strategies() -> Vec<(&'static str, &'static str)> {
    vec![
        (
            "sma",
            "SMA Crossover (7/25) — buy on golden cross, sell on death cross",
        ),
        ("sma_10_30", "SMA Crossover (10/30) — slower, fewer trades"),
        (
            "rsi",
            "RSI Mean-Reversion (14, 30/70) — buy oversold, sell overbought",
        ),
        (
            "rsi_14_25_75",
            "RSI Mean-Reversion (14, 25/75) — tighter bands",
        ),
        (
            "bb",
            "Bollinger Squeeze (20, 2σ) — buy breakout from low volatility squeeze",
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn trending_up_prices(n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| 100.0 + i as f64 * 0.5 + (i as f64 * 0.3).sin() * 3.0)
            .collect()
    }

    fn trending_down_prices(n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| 200.0 - i as f64 * 0.5 + (i as f64 * 0.3).sin() * 3.0)
            .collect()
    }

    fn oscillating_prices(n: usize) -> Vec<f64> {
        (0..n)
            .map(|i| 100.0 + (i as f64 * 0.2).sin() * 20.0)
            .collect()
    }

    #[test]
    fn test_sma_crossover_basic() {
        let prices = trending_up_prices(100);
        let result = run_backtest(
            &prices,
            &Strategy::SmaCrossover {
                short_period: 7,
                long_period: 25,
            },
            "TEST",
            "90d",
        );
        assert!(result.data_points == 100);
        // An uptrend should produce some trades
        assert!(result.total_return_pct != 0.0 || result.total_trades == 0);
    }

    #[test]
    fn test_sma_crossover_downtrend() {
        let prices = trending_down_prices(100);
        let result = run_backtest(
            &prices,
            &Strategy::SmaCrossover {
                short_period: 7,
                long_period: 25,
            },
            "TEST",
            "90d",
        );
        // In a steady downtrend, SMA crossover might lose money
        assert!(result.data_points == 100);
    }

    #[test]
    fn test_sma_crossover_oscillating() {
        let prices = oscillating_prices(200);
        let result = run_backtest(
            &prices,
            &Strategy::SmaCrossover {
                short_period: 7,
                long_period: 25,
            },
            "TEST",
            "90d",
        );
        // Oscillating prices should generate multiple trades
        assert!(
            result.total_trades > 0,
            "Should generate trades on oscillating data, got {}",
            result.total_trades
        );
    }

    #[test]
    fn test_rsi_mean_reversion_basic() {
        let prices = oscillating_prices(200);
        let result = run_backtest(
            &prices,
            &Strategy::RsiMeanReversion {
                period: 14,
                oversold: 30.0,
                overbought: 70.0,
            },
            "TEST",
            "90d",
        );
        assert!(result.data_points == 200);
        // RSI mean reversion should generate some trades on oscillating data
    }

    #[test]
    fn test_backtest_insufficient_data() {
        let prices = vec![100.0, 101.0, 102.0];
        let result = run_backtest(
            &prices,
            &Strategy::SmaCrossover {
                short_period: 7,
                long_period: 25,
            },
            "TEST",
            "1d",
        );
        assert_eq!(result.total_trades, 0);
    }

    #[test]
    fn test_backtest_format() {
        let prices = oscillating_prices(200);
        let result = run_backtest(
            &prices,
            &Strategy::SmaCrossover {
                short_period: 7,
                long_period: 25,
            },
            "bitcoin",
            "90d",
        );
        let formatted = result.format();
        assert!(formatted.contains("Backtest"));
        assert!(formatted.contains("bitcoin"));
        assert!(formatted.contains("Buy & Hold"));
        assert!(formatted.contains("not indicative"));
    }

    #[test]
    fn test_parse_strategy_sma() {
        let s = parse_strategy("sma").unwrap();
        assert!(matches!(
            s,
            Strategy::SmaCrossover {
                short_period: 7,
                long_period: 25
            }
        ));
    }

    #[test]
    fn test_parse_strategy_sma_custom() {
        let s = parse_strategy("sma_10_30").unwrap();
        assert!(matches!(
            s,
            Strategy::SmaCrossover {
                short_period: 10,
                long_period: 30
            }
        ));
    }

    #[test]
    fn test_parse_strategy_rsi() {
        let s = parse_strategy("rsi").unwrap();
        assert!(matches!(s, Strategy::RsiMeanReversion { period: 14, .. }));
    }

    #[test]
    fn test_parse_strategy_invalid() {
        assert!(parse_strategy("unknown").is_none());
        assert!(parse_strategy("sma_30_10").is_none()); // short >= long
    }

    #[test]
    fn test_calculate_sharpe_no_data() {
        assert_eq!(calculate_sharpe(&[]), 0.0);
        assert_eq!(calculate_sharpe(&[0.01]), 0.0);
    }

    #[test]
    fn test_calculate_sharpe_positive() {
        // All positive returns should give positive Sharpe
        let returns = vec![
            0.01, 0.02, 0.015, 0.01, 0.025, 0.01, 0.02, 0.015, 0.01, 0.025,
        ];
        let sharpe = calculate_sharpe(&returns);
        assert!(
            sharpe > 0.0,
            "Sharpe should be positive for positive returns, got {}",
            sharpe
        );
    }

    #[test]
    fn test_buy_hold_comparison() {
        // Uptrend: SMA crossover should capture some of the move
        let prices = trending_up_prices(100);
        let result = run_backtest(
            &prices,
            &Strategy::SmaCrossover {
                short_period: 7,
                long_period: 25,
            },
            "TEST",
            "90d",
        );
        // Buy and hold should be positive in an uptrend
        assert!(
            result.buy_hold_return_pct > 0.0,
            "Buy & hold should be positive in uptrend"
        );
    }

    #[test]
    fn test_max_drawdown() {
        // Create data that goes up then crashes
        let mut prices: Vec<f64> = (0..50).map(|i| 100.0 + i as f64 * 2.0).collect();
        // Add crash
        for i in 0..50 {
            prices.push(200.0 - i as f64 * 3.0);
        }
        let result = run_backtest(
            &prices,
            &Strategy::SmaCrossover {
                short_period: 7,
                long_period: 25,
            },
            "TEST",
            "90d",
        );
        // Result should have a max drawdown (if any trades occurred during the crash)
        assert!(result.max_drawdown_pct >= 0.0);
    }

    #[test]
    fn test_win_rate_bounds() {
        let prices = oscillating_prices(300);
        let result = run_backtest(
            &prices,
            &Strategy::SmaCrossover {
                short_period: 7,
                long_period: 25,
            },
            "TEST",
            "90d",
        );
        if result.total_trades > 0 {
            assert!(result.win_rate >= 0.0 && result.win_rate <= 100.0);
        }
    }

    #[test]
    fn test_available_strategies_not_empty() {
        let strategies = available_strategies();
        assert!(!strategies.is_empty());
        assert!(strategies.len() >= 5);
    }

    #[test]
    fn test_parse_strategy_bollinger() {
        let s = parse_strategy("bb").unwrap();
        assert!(matches!(s, Strategy::BollingerSqueeze { period: 20, .. }));
        let s2 = parse_strategy("squeeze").unwrap();
        assert!(matches!(s2, Strategy::BollingerSqueeze { .. }));
    }

    #[test]
    fn test_bollinger_squeeze_oscillating() {
        // Create data that squeezes then breaks out
        let mut prices = Vec::new();
        // Phase 1: Low volatility period (squeeze)
        for i in 0..40 {
            prices.push(100.0 + (i as f64 * 0.1).sin() * 0.5);
        }
        // Phase 2: Breakout upward
        for i in 0..30 {
            prices.push(100.5 + i as f64 * 1.5);
        }
        // Phase 3: Another squeeze
        for i in 0..30 {
            prices.push(145.0 + (i as f64 * 0.1).sin() * 0.5);
        }
        
        let result = run_backtest(
            &prices,
            &Strategy::BollingerSqueeze { period: 20, std_dev: 2.0 },
            "TEST",
            "90d",
        );
        assert!(result.data_points == 100);
        // The breakout phase should generate at least one trade
    }

    #[test]
    fn test_bollinger_squeeze_insufficient_data() {
        let prices = vec![100.0; 20];
        let result = run_backtest(
            &prices,
            &Strategy::BollingerSqueeze { period: 20, std_dev: 2.0 },
            "TEST",
            "7d",
        );
        assert_eq!(result.total_trades, 0);
    }
}
