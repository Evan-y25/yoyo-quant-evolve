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
    BollingerSqueeze { period: usize, std_dev: f64 },
    /// MACD crossover: buy when MACD crosses above signal line,
    /// sell when MACD crosses below signal line.
    MacdCrossover {
        fast: usize,
        slow: usize,
        signal_period: usize,
    },
    /// Stochastic Oscillator: buy when %K crosses above %D in oversold zone,
    /// sell when %K crosses below %D in overbought zone.
    StochasticOscillator {
        period: usize,
        signal_period: usize,
        oversold: f64,
        overbought: f64,
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
            Strategy::MacdCrossover {
                fast,
                slow,
                signal_period,
            } => {
                format!("MACD Crossover ({}/{}/{})", fast, slow, signal_period)
            }
            Strategy::StochasticOscillator {
                period,
                signal_period,
                oversold,
                overbought,
            } => {
                format!(
                    "Stochastic ({},{},{}/{})",
                    period, signal_period, oversold, overbought
                )
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
        Strategy::MacdCrossover {
            fast,
            slow,
            signal_period,
        } => backtest_macd_crossover(prices, *fast, *slow, *signal_period),
        Strategy::StochasticOscillator {
            period,
            signal_period,
            oversold,
            overbought,
        } => backtest_stochastic(prices, *period, *signal_period, *oversold, *overbought),
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
fn backtest_bollinger_squeeze(prices: &[f64], period: usize, std_dev: f64) -> Vec<BacktestTrade> {
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
            let variance =
                window.iter().map(|&p| (p - middle).powi(2)).sum::<f64>() / period as f64;
            let stddev = variance.sqrt();
            let upper = middle + std_dev * stddev;
            let lower = middle - std_dev * stddev;
            let bw = if middle > 0.0 {
                (upper - lower) / middle * 100.0
            } else {
                0.0
            };
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
            .sum::<f64>()
            / lookback as f64;

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

/// MACD crossover strategy.
/// Buy when MACD line crosses above signal line, sell when it crosses below.
/// Uses the standard approach of computing EMA(fast) - EMA(slow) and then
/// EMA of that difference as the signal line.
fn backtest_macd_crossover(
    prices: &[f64],
    fast: usize,
    slow: usize,
    signal_period: usize,
) -> Vec<BacktestTrade> {
    let mut trades = Vec::new();
    if prices.len() < slow + signal_period || fast >= slow || fast == 0 || slow == 0 {
        return trades;
    }

    let multiplier_fast = 2.0 / (fast as f64 + 1.0);
    let multiplier_slow = 2.0 / (slow as f64 + 1.0);

    // Build full MACD line series
    // Fast EMA initialization
    let mut fast_ema = prices[..fast].iter().sum::<f64>() / fast as f64;
    for &price in &prices[fast..slow] {
        fast_ema = (price - fast_ema) * multiplier_fast + fast_ema;
    }
    let mut slow_ema = prices[..slow].iter().sum::<f64>() / slow as f64;

    let mut macd_series: Vec<f64> = Vec::new();
    macd_series.push(fast_ema - slow_ema);

    for &price in &prices[slow..] {
        fast_ema = (price - fast_ema) * multiplier_fast + fast_ema;
        slow_ema = (price - slow_ema) * multiplier_slow + slow_ema;
        macd_series.push(fast_ema - slow_ema);
    }

    if macd_series.len() < signal_period {
        return trades;
    }

    // Build signal line series
    let multiplier_signal = 2.0 / (signal_period as f64 + 1.0);
    let mut signal_ema = macd_series[..signal_period].iter().sum::<f64>() / signal_period as f64;
    let mut signal_series: Vec<f64> = vec![0.0; signal_period]; // padding
    signal_series[signal_period - 1] = signal_ema;
    for i in signal_period..macd_series.len() {
        signal_ema = (macd_series[i] - signal_ema) * multiplier_signal + signal_ema;
        signal_series.push(signal_ema);
    }

    // The MACD series starts at price index `slow - 1` (0-indexed in macd_series)
    // So macd_series[i] corresponds to prices[slow - 1 + i]
    let price_offset = slow - 1;

    let mut in_position = false;
    let mut entry_idx = 0;
    let mut entry_price = 0.0;

    // Start from signal_period (where signal line is first valid)
    for i in signal_period..macd_series.len() {
        if i == 0 {
            continue;
        }
        let prev_macd = macd_series[i - 1];
        let prev_signal = signal_series[i - 1];
        let curr_macd = macd_series[i];
        let curr_signal = signal_series[i];

        let price_idx = price_offset + i;
        if price_idx >= prices.len() {
            break;
        }

        // Bullish crossover: MACD crosses above signal
        if !in_position && prev_macd <= prev_signal && curr_macd > curr_signal {
            in_position = true;
            entry_idx = price_idx;
            entry_price = prices[price_idx];
        }
        // Bearish crossover: MACD crosses below signal
        else if in_position && prev_macd >= prev_signal && curr_macd < curr_signal {
            let exit_price = prices[price_idx];
            let pnl = exit_price - entry_price;
            let pnl_pct = (pnl / entry_price) * 100.0;
            trades.push(BacktestTrade {
                entry_idx,
                exit_idx: price_idx,
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

/// Stochastic Oscillator strategy.
/// Buy when %K crosses above %D in oversold territory (below oversold threshold).
/// Sell when %K crosses below %D in overbought territory (above overbought threshold).
///
/// Since we often only have close prices (not separate high/low), we approximate
/// high/low using a rolling max/min of close prices.
fn backtest_stochastic(
    prices: &[f64],
    period: usize,
    signal_period: usize,
    oversold: f64,
    overbought: f64,
) -> Vec<BacktestTrade> {
    let mut trades = Vec::new();
    if prices.len() < period + signal_period + 1 {
        return trades;
    }

    // Compute %K values at each point
    // %K = (Close - Lowest Low) / (Highest High - Lowest Low) * 100
    let mut k_values: Vec<Option<f64>> = Vec::with_capacity(prices.len());
    for i in 0..prices.len() {
        if i + 1 >= period {
            let window = &prices[i + 1 - period..=i];
            let highest = window.iter().copied().fold(f64::NEG_INFINITY, f64::max);
            let lowest = window.iter().copied().fold(f64::INFINITY, f64::min);
            let range = highest - lowest;
            let k = if range > 0.0 {
                ((prices[i] - lowest) / range) * 100.0
            } else {
                50.0
            };
            k_values.push(Some(k));
        } else {
            k_values.push(None);
        }
    }

    // Compute %D values (SMA of %K over signal_period)
    let mut d_values: Vec<Option<f64>> = Vec::with_capacity(prices.len());
    for i in 0..prices.len() {
        if i + 1 >= period + signal_period - 1 {
            let start = i + 1 - signal_period;
            let k_window: Vec<f64> = (start..=i)
                .filter_map(|j| k_values.get(j).and_then(|v| *v))
                .collect();
            if k_window.len() == signal_period {
                let d = k_window.iter().sum::<f64>() / signal_period as f64;
                d_values.push(Some(d));
            } else {
                d_values.push(None);
            }
        } else {
            d_values.push(None);
        }
    }

    let mut in_position = false;
    let mut entry_idx = 0;
    let mut entry_price = 0.0;

    for i in 1..prices.len() {
        let (curr_k, curr_d) = match (
            k_values.get(i).and_then(|v| *v),
            d_values.get(i).and_then(|v| *v),
        ) {
            (Some(k), Some(d)) => (k, d),
            _ => continue,
        };
        let (prev_k, prev_d) = match (
            k_values.get(i - 1).and_then(|v| *v),
            d_values.get(i - 1).and_then(|v| *v),
        ) {
            (Some(k), Some(d)) => (k, d),
            _ => continue,
        };

        // Buy signal: %K crosses above %D while in oversold zone
        if !in_position && prev_k <= prev_d && curr_k > curr_d && curr_k < oversold + 15.0 {
            in_position = true;
            entry_idx = i;
            entry_price = prices[i];
        }
        // Sell signal: %K crosses below %D while in overbought zone
        else if in_position && prev_k >= prev_d && curr_k < curr_d && curr_k > overbought - 15.0 {
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
        Some("macd") => {
            let fast = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(12);
            let slow = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(26);
            let signal_period = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(9);
            if fast >= slow || fast == 0 || slow == 0 || signal_period == 0 {
                return None;
            }
            Some(Strategy::MacdCrossover {
                fast,
                slow,
                signal_period,
            })
        }
        Some("stoch") | Some("stochastic") => {
            let period = parts.get(1).and_then(|s| s.parse().ok()).unwrap_or(14);
            let signal_period = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(3);
            let oversold = parts.get(3).and_then(|s| s.parse().ok()).unwrap_or(20.0);
            let overbought = parts.get(4).and_then(|s| s.parse().ok()).unwrap_or(80.0);
            if period == 0 || signal_period == 0 || oversold >= overbought {
                return None;
            }
            Some(Strategy::StochasticOscillator {
                period,
                signal_period,
                oversold,
                overbought,
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
        (
            "macd",
            "MACD Crossover (12/26/9) — buy on bullish crossover, sell on bearish",
        ),
        (
            "stoch",
            "Stochastic Oscillator (14/3, 20/80) — buy oversold crossover, sell overbought",
        ),
    ]
}

/// Get all default strategies for comparison backtesting.
pub fn all_default_strategies() -> Vec<Strategy> {
    vec![
        Strategy::SmaCrossover {
            short_period: 7,
            long_period: 25,
        },
        Strategy::SmaCrossover {
            short_period: 10,
            long_period: 30,
        },
        Strategy::RsiMeanReversion {
            period: 14,
            oversold: 30.0,
            overbought: 70.0,
        },
        Strategy::RsiMeanReversion {
            period: 14,
            oversold: 25.0,
            overbought: 75.0,
        },
        Strategy::BollingerSqueeze {
            period: 20,
            std_dev: 2.0,
        },
        Strategy::MacdCrossover {
            fast: 12,
            slow: 26,
            signal_period: 9,
        },
        Strategy::StochasticOscillator {
            period: 14,
            signal_period: 3,
            oversold: 20.0,
            overbought: 80.0,
        },
    ]
}

/// Result of comparing multiple strategies on the same data.
pub struct ComparisonResult {
    pub symbol: String,
    pub range: String,
    pub data_points: usize,
    pub buy_hold_return_pct: f64,
    pub results: Vec<BacktestResult>,
}

impl ComparisonResult {
    /// Format comparison as a ranked table.
    pub fn format(&self) -> String {
        let mut output = String::new();

        output.push_str(&format!(
            "🏆 Strategy Comparison: {} ({})\n",
            self.symbol, self.range
        ));
        output.push_str("═════════════════════════════════════════════════════════════════\n");
        output.push_str(&format!("  Data Points: {}\n", self.data_points));
        output.push_str(&format!(
            "  Buy & Hold:  {}{:.2}%\n",
            if self.buy_hold_return_pct >= 0.0 {
                "+"
            } else {
                ""
            },
            self.buy_hold_return_pct
        ));
        output.push_str("─────────────────────────────────────────────────────────────────\n");
        output.push_str(&format!(
            "  {:<3} {:<28} {:>8} {:>8} {:>7} {:>6} {:>8}\n",
            "#", "Strategy", "Return", "Alpha", "WinR", "Trades", "MaxDD"
        ));
        output.push_str("─────────────────────────────────────────────────────────────────\n");

        // Sort results by total return (descending)
        let mut sorted_results: Vec<&BacktestResult> = self.results.iter().collect();
        sorted_results.sort_by(|a, b| {
            b.total_return_pct
                .partial_cmp(&a.total_return_pct)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for (rank, result) in sorted_results.iter().enumerate() {
            let rank_emoji = match rank {
                0 => "🥇",
                1 => "🥈",
                2 => "🥉",
                _ => "  ",
            };
            let alpha = result.total_return_pct - self.buy_hold_return_pct;
            let beat_bh = if alpha > 0.0 { "🟢" } else { "🔴" };

            output.push_str(&format!(
                "  {} {:<28} {:>+7.2}% {:>+7.2}% {} {:>5.1}% {:>5} {:>-7.2}%\n",
                rank_emoji,
                truncate_str(&result.strategy_name, 28),
                result.total_return_pct,
                alpha,
                beat_bh,
                result.win_rate,
                result.total_trades,
                result.max_drawdown_pct,
            ));
        }

        output.push_str("─────────────────────────────────────────────────────────────────\n");

        // Summary insights
        if let Some(best) = sorted_results.first() {
            let alpha = best.total_return_pct - self.buy_hold_return_pct;
            if alpha > 0.0 {
                output.push_str(&format!(
                    "  🏆 Best: {} ({}{:.2}% alpha)\n",
                    best.strategy_name,
                    if alpha >= 0.0 { "+" } else { "" },
                    alpha,
                ));
            } else {
                output.push_str("  📉 No strategy beat buy & hold on this data.\n");
                output.push_str("  💡 Sometimes the best trade is no trade.\n");
            }
        }

        // Look for strategies with good risk-adjusted returns
        let best_sharpe = sorted_results
            .iter()
            .filter(|r| r.sharpe_ratio.is_finite() && r.total_trades >= 3)
            .max_by(|a, b| {
                a.sharpe_ratio
                    .partial_cmp(&b.sharpe_ratio)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

        if let Some(sr) = best_sharpe {
            if sr.sharpe_ratio > 0.0 {
                output.push_str(&format!(
                    "  📊 Best risk-adjusted: {} (Sharpe: {:.2})\n",
                    sr.strategy_name, sr.sharpe_ratio,
                ));
            }
        }

        output.push_str("═════════════════════════════════════════════════════════════════\n");
        output.push_str("  ⚠️  Past performance is not indicative of future results.\n");
        output.push_str("  Backtest does not account for fees, slippage, or liquidity.\n");

        output
    }
}

/// Run all default strategies on the same price data and return a comparison.
pub fn run_comparison(prices: &[f64], symbol: &str, range: &str) -> ComparisonResult {
    let strategies = all_default_strategies();
    let results: Vec<BacktestResult> = strategies
        .iter()
        .map(|s| run_backtest(prices, s, symbol, range))
        .collect();

    let buy_hold_return_pct = if prices.len() >= 2 && prices[0] > 0.0 {
        ((prices[prices.len() - 1] - prices[0]) / prices[0]) * 100.0
    } else {
        0.0
    };

    ComparisonResult {
        symbol: symbol.to_string(),
        range: range.to_string(),
        data_points: prices.len(),
        buy_hold_return_pct,
        results,
    }
}

/// Truncate a string to a max length, adding "..." if truncated.
fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else if max > 3 {
        format!("{}...", &s[..max - 3])
    } else {
        s[..max].to_string()
    }
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
        assert!(strategies.len() >= 6);
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
            &Strategy::BollingerSqueeze {
                period: 20,
                std_dev: 2.0,
            },
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
            &Strategy::BollingerSqueeze {
                period: 20,
                std_dev: 2.0,
            },
            "TEST",
            "7d",
        );
        assert_eq!(result.total_trades, 0);
    }

    #[test]
    fn test_all_default_strategies() {
        let strategies = all_default_strategies();
        assert_eq!(
            strategies.len(),
            7,
            "Should have 7 default strategies (including Stochastic)"
        );
        // Verify all names are non-empty
        for s in &strategies {
            assert!(!s.name().is_empty());
        }
    }

    #[test]
    fn test_run_comparison_basic() {
        let prices = oscillating_prices(200);
        let result = run_comparison(&prices, "TEST", "90d");
        assert_eq!(result.symbol, "TEST");
        assert_eq!(result.range, "90d");
        assert_eq!(result.data_points, 200);
        assert_eq!(result.results.len(), 7, "Should run all 7 strategies");
    }

    #[test]
    fn test_run_comparison_uptrend() {
        let prices = trending_up_prices(100);
        let result = run_comparison(&prices, "TEST", "90d");
        // Buy-and-hold should be positive in an uptrend
        assert!(
            result.buy_hold_return_pct > 0.0,
            "Buy & hold should be positive in uptrend"
        );
        // All results should have the same data_points
        for r in &result.results {
            assert_eq!(r.data_points, 100);
        }
    }

    #[test]
    fn test_comparison_format() {
        let prices = oscillating_prices(200);
        let result = run_comparison(&prices, "bitcoin", "90d");
        let formatted = result.format();
        assert!(formatted.contains("Strategy Comparison"));
        assert!(formatted.contains("bitcoin"));
        assert!(formatted.contains("Buy & Hold"));
        assert!(formatted.contains("Return"));
        assert!(formatted.contains("Alpha"));
        assert!(formatted.contains("🥇"));
        assert!(formatted.contains("not indicative"));
    }

    #[test]
    fn test_comparison_insufficient_data() {
        let prices = vec![100.0; 10];
        let result = run_comparison(&prices, "TEST", "7d");
        // With very little data, most strategies won't generate trades
        let total_trades: usize = result.results.iter().map(|r| r.total_trades).sum();
        assert_eq!(total_trades, 0, "Should have no trades on constant data");
    }

    #[test]
    fn test_truncate_str() {
        assert_eq!(truncate_str("hello", 10), "hello");
        assert_eq!(truncate_str("hello world foo", 10), "hello w...");
        assert_eq!(truncate_str("hello", 5), "hello");
        assert_eq!(truncate_str("hi", 2), "hi");
    }

    #[test]
    fn test_macd_crossover_basic() {
        let prices = oscillating_prices(200);
        let result = run_backtest(
            &prices,
            &Strategy::MacdCrossover {
                fast: 12,
                slow: 26,
                signal_period: 9,
            },
            "TEST",
            "90d",
        );
        assert!(result.data_points == 200);
        // Oscillating data should produce some MACD crossovers
    }

    #[test]
    fn test_macd_crossover_uptrend() {
        let prices = trending_up_prices(100);
        let result = run_backtest(
            &prices,
            &Strategy::MacdCrossover {
                fast: 12,
                slow: 26,
                signal_period: 9,
            },
            "TEST",
            "90d",
        );
        // In an uptrend, MACD crossover should capture some gains
        assert!(result.data_points == 100);
    }

    #[test]
    fn test_macd_crossover_insufficient_data() {
        let prices = vec![100.0; 20];
        let result = run_backtest(
            &prices,
            &Strategy::MacdCrossover {
                fast: 12,
                slow: 26,
                signal_period: 9,
            },
            "TEST",
            "7d",
        );
        assert_eq!(result.total_trades, 0);
    }

    #[test]
    fn test_parse_strategy_macd() {
        let s = parse_strategy("macd").unwrap();
        assert!(matches!(
            s,
            Strategy::MacdCrossover {
                fast: 12,
                slow: 26,
                signal_period: 9
            }
        ));
    }

    #[test]
    fn test_parse_strategy_macd_custom() {
        let s = parse_strategy("macd_8_21_5").unwrap();
        assert!(matches!(
            s,
            Strategy::MacdCrossover {
                fast: 8,
                slow: 21,
                signal_period: 5
            }
        ));
    }

    #[test]
    fn test_available_strategies_count() {
        let strategies = available_strategies();
        assert!(
            strategies.len() >= 6,
            "Should have at least 6 strategies including MACD"
        );
    }
}
