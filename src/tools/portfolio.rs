//! Paper trading portfolio — track simulated trades and positions.
//!
//! This is the foundation for Level 3 (trading). All trades are paper trades
//! (simulated) — no real money is at risk. Each trade is logged with reasoning,
//! entry price, and optional exit for P&L tracking.
//!
//! Portfolio state is persisted to portfolio.json in the current directory.

use serde::{Deserialize, Serialize};
use std::path::Path;

const PORTFOLIO_FILE: &str = "portfolio.json";
const DEFAULT_STARTING_BALANCE: f64 = 100_000.0;

/// Stats aggregated per symbol for the performance dashboard.
struct SymbolStats {
    total_trades: u32,
    wins: u32,
    losses: u32,
    total_pnl: f64,
    best_pnl: f64,
    worst_pnl: f64,
}

/// A single paper trade.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperTrade {
    /// Unique trade ID (auto-incremented)
    pub id: u32,
    /// Asset symbol (e.g., "bitcoin", "AAPL")
    pub symbol: String,
    /// "buy" or "sell" (short)
    pub side: String,
    /// Number of units
    pub quantity: f64,
    /// Price at entry
    pub entry_price: f64,
    /// Price at exit (None if position is still open)
    pub exit_price: Option<f64>,
    /// Reasoning for the trade
    pub reasoning: String,
    /// Confidence level 1-10
    pub confidence: u8,
    /// ISO 8601 timestamp of entry
    pub entry_time: String,
    /// ISO 8601 timestamp of exit
    pub exit_time: Option<String>,
    /// Realized P&L (None if still open)
    pub realized_pnl: Option<f64>,
    /// Stop-loss price (auto-close if price hits this level)
    #[serde(default)]
    pub stop_loss: Option<f64>,
    /// Take-profit price (auto-close if price hits this level)
    #[serde(default)]
    pub take_profit: Option<f64>,
    /// Trailing stop distance as a percentage (e.g., 5.0 = 5%).
    /// When set, the stop-loss automatically ratchets upward (for buys)
    /// or downward (for shorts) as the price moves favorably.
    #[serde(default)]
    pub trailing_stop_pct: Option<f64>,
    /// The highest price seen since opening (for trailing stop on buys).
    /// Updated each time prices are checked.
    #[serde(default)]
    pub highest_price_seen: Option<f64>,
    /// The lowest price seen since opening (for trailing stop on shorts).
    /// Updated each time prices are checked.
    #[serde(default)]
    pub lowest_price_seen: Option<f64>,
}

impl PaperTrade {
    /// Calculate the notional value at entry.
    pub fn notional_value(&self) -> f64 {
        self.quantity * self.entry_price
    }

    /// Calculate unrealized P&L given a current price.
    pub fn unrealized_pnl(&self, current_price: f64) -> f64 {
        if self.exit_price.is_some() {
            return self.realized_pnl.unwrap_or(0.0);
        }
        let direction = if self.side == "buy" { 1.0 } else { -1.0 };
        direction * self.quantity * (current_price - self.entry_price)
    }

    /// Check if position is still open.
    pub fn is_open(&self) -> bool {
        self.exit_price.is_none()
    }

    /// Calculate P&L percentage.
    pub fn pnl_pct(&self, current_price: f64) -> f64 {
        let pnl = self.unrealized_pnl(current_price);
        let entry_notional = self.notional_value();
        if entry_notional > 0.0 {
            (pnl / entry_notional) * 100.0
        } else {
            0.0
        }
    }
}

/// The paper trading portfolio.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Portfolio {
    /// Starting cash balance
    pub starting_balance: f64,
    /// Current cash balance (decreases on buys, increases on sells)
    pub cash: f64,
    /// All trades (open and closed)
    pub trades: Vec<PaperTrade>,
    /// Next trade ID
    pub next_id: u32,
}

impl Portfolio {
    /// Create a new empty portfolio with default balance.
    pub fn new() -> Self {
        Self {
            starting_balance: DEFAULT_STARTING_BALANCE,
            cash: DEFAULT_STARTING_BALANCE,
            trades: Vec::new(),
            next_id: 1,
        }
    }

    /// Load portfolio from disk, or return a new one.
    pub fn load() -> Self {
        let path = Path::new(PORTFOLIO_FILE);
        if !path.exists() {
            return Self::new();
        }

        match std::fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| Self::new()),
            Err(_) => Self::new(),
        }
    }

    /// Save portfolio to disk.
    pub fn save(&self) -> Result<(), String> {
        let json =
            serde_json::to_string_pretty(self).map_err(|e| format!("Serialize error: {}", e))?;
        std::fs::write(PORTFOLIO_FILE, json).map_err(|e| format!("Write error: {}", e))
    }

    /// Open a new paper trade (buy or sell).
    /// Returns the trade ID on success.
    pub fn open_trade(
        &mut self,
        symbol: &str,
        side: &str,
        quantity: f64,
        price: f64,
        reasoning: &str,
        confidence: u8,
    ) -> Result<u32, String> {
        self.open_trade_with_levels(
            symbol, side, quantity, price, reasoning, confidence, None, None,
        )
    }

    /// Open a new paper trade with optional stop-loss and take-profit.
    /// Returns the trade ID on success.
    pub fn open_trade_with_levels(
        &mut self,
        symbol: &str,
        side: &str,
        quantity: f64,
        price: f64,
        reasoning: &str,
        confidence: u8,
        stop_loss: Option<f64>,
        take_profit: Option<f64>,
    ) -> Result<u32, String> {
        if quantity <= 0.0 {
            return Err("Quantity must be positive".into());
        }
        if price <= 0.0 {
            return Err("Price must be positive".into());
        }
        if side != "buy" && side != "sell" {
            return Err("Side must be 'buy' or 'sell'".into());
        }
        if confidence > 10 {
            return Err("Confidence must be 1-10".into());
        }

        let cost = quantity * price;
        if side == "buy" && cost > self.cash {
            return Err(format!(
                "Insufficient cash. Need ${:.2} but only have ${:.2}",
                cost, self.cash
            ));
        }

        // Deduct cash for buys
        if side == "buy" {
            self.cash -= cost;
        }

        // Validate stop-loss and take-profit
        if let Some(sl) = stop_loss {
            if sl <= 0.0 {
                return Err("Stop-loss must be positive".into());
            }
            if side == "buy" && sl >= price {
                return Err(format!(
                    "Stop-loss (${:.2}) must be below entry price (${:.2}) for a buy",
                    sl, price
                ));
            }
            if side == "sell" && sl <= price {
                return Err(format!(
                    "Stop-loss (${:.2}) must be above entry price (${:.2}) for a short",
                    sl, price
                ));
            }
        }
        if let Some(tp) = take_profit {
            if tp <= 0.0 {
                return Err("Take-profit must be positive".into());
            }
            if side == "buy" && tp <= price {
                return Err(format!(
                    "Take-profit (${:.2}) must be above entry price (${:.2}) for a buy",
                    tp, price
                ));
            }
            if side == "sell" && tp >= price {
                return Err(format!(
                    "Take-profit (${:.2}) must be below entry price (${:.2}) for a short",
                    tp, price
                ));
            }
        }

        let now = current_timestamp();
        let id = self.next_id;
        self.next_id += 1;

        let trade = PaperTrade {
            id,
            symbol: symbol.to_string(),
            side: side.to_string(),
            quantity,
            entry_price: price,
            exit_price: None,
            reasoning: reasoning.to_string(),
            confidence,
            entry_time: now,
            exit_time: None,
            realized_pnl: None,
            stop_loss,
            take_profit,
            trailing_stop_pct: None,
            highest_price_seen: Some(price),
            lowest_price_seen: Some(price),
        };

        self.trades.push(trade);
        Ok(id)
    }

    /// Close an open trade at a given price.
    pub fn close_trade(&mut self, trade_id: u32, exit_price: f64) -> Result<f64, String> {
        if exit_price <= 0.0 {
            return Err("Exit price must be positive".into());
        }

        let trade = self
            .trades
            .iter_mut()
            .find(|t| t.id == trade_id && t.is_open())
            .ok_or_else(|| format!("No open trade found with ID #{}", trade_id))?;

        let direction = if trade.side == "buy" { 1.0 } else { -1.0 };
        let pnl = direction * trade.quantity * (exit_price - trade.entry_price);

        trade.exit_price = Some(exit_price);
        trade.exit_time = Some(current_timestamp());
        trade.realized_pnl = Some(pnl);

        // Return cash: original cost + P&L
        if trade.side == "buy" {
            self.cash += trade.quantity * exit_price;
        } else {
            self.cash += pnl; // For shorts, only add the P&L
        }

        Ok(pnl)
    }

    /// Check if any open trades should be closed based on stop-loss, trailing stop, or take-profit.
    /// Also updates trailing stop state (highest/lowest price seen, ratcheted SL).
    /// Returns a list of (trade_id, trigger_price, trigger_type) for trades that should close.
    pub fn check_stop_loss_take_profit(
        &mut self,
        price_map: &std::collections::HashMap<String, f64>,
    ) -> Vec<(u32, f64, &'static str)> {
        let mut triggered = Vec::new();

        for trade in &mut self.trades {
            if !trade.is_open() {
                continue;
            }
            if let Some(&current_price) = price_map.get(&trade.symbol) {
                // Update trailing stop if configured
                if let Some(trail_pct) = trade.trailing_stop_pct {
                    if trade.side == "buy" {
                        // Track highest price seen
                        let highest = trade.highest_price_seen.unwrap_or(trade.entry_price);
                        if current_price > highest {
                            trade.highest_price_seen = Some(current_price);
                        }
                        let best = trade.highest_price_seen.unwrap_or(trade.entry_price);
                        // Trailing SL = best price * (1 - trail_pct/100)
                        let trailing_sl = best * (1.0 - trail_pct / 100.0);
                        // Only ratchet upward — never lower the stop
                        let current_sl = trade.stop_loss.unwrap_or(0.0);
                        if trailing_sl > current_sl {
                            trade.stop_loss = Some(trailing_sl);
                        }
                    } else {
                        // Short: track lowest price seen
                        let lowest = trade.lowest_price_seen.unwrap_or(trade.entry_price);
                        if current_price < lowest {
                            trade.lowest_price_seen = Some(current_price);
                        }
                        let best = trade.lowest_price_seen.unwrap_or(trade.entry_price);
                        // Trailing SL = best price * (1 + trail_pct/100)
                        let trailing_sl = best * (1.0 + trail_pct / 100.0);
                        // Only ratchet downward — never raise the stop for shorts
                        let current_sl = trade.stop_loss.unwrap_or(f64::MAX);
                        if trailing_sl < current_sl {
                            trade.stop_loss = Some(trailing_sl);
                        }
                    }
                }

                // Check stop-loss (including trailing stop)
                if let Some(sl) = trade.stop_loss {
                    let triggered_sl = if trade.side == "buy" {
                        current_price <= sl
                    } else {
                        current_price >= sl
                    };
                    if triggered_sl {
                        let trigger_type = if trade.trailing_stop_pct.is_some() {
                            "trailing-stop"
                        } else {
                            "stop-loss"
                        };
                        triggered.push((trade.id, current_price, trigger_type));
                        continue; // Don't check TP if SL triggered
                    }
                }
                // Check take-profit
                if let Some(tp) = trade.take_profit {
                    let triggered_tp = if trade.side == "buy" {
                        current_price >= tp
                    } else {
                        current_price <= tp
                    };
                    if triggered_tp {
                        triggered.push((trade.id, current_price, "take-profit"));
                    }
                }
            }
        }

        triggered
    }

    /// Get all open positions.
    pub fn open_positions(&self) -> Vec<&PaperTrade> {
        self.trades.iter().filter(|t| t.is_open()).collect()
    }

    /// Get all closed positions.
    pub fn closed_positions(&self) -> Vec<&PaperTrade> {
        self.trades.iter().filter(|t| !t.is_open()).collect()
    }

    /// Calculate total realized P&L from closed trades.
    pub fn total_realized_pnl(&self) -> f64 {
        self.trades.iter().filter_map(|t| t.realized_pnl).sum()
    }

    /// Calculate win rate from closed trades.
    pub fn win_rate(&self) -> Option<f64> {
        let closed = self.closed_positions();
        if closed.is_empty() {
            return None;
        }
        let wins = closed
            .iter()
            .filter(|t| t.realized_pnl.unwrap_or(0.0) > 0.0)
            .count();
        Some((wins as f64 / closed.len() as f64) * 100.0)
    }

    /// Get portfolio summary with live prices for unrealized P&L.
    pub fn summary_with_prices(
        &self,
        price_map: &std::collections::HashMap<String, f64>,
    ) -> String {
        use super::format::{format_currency, format_currency_unsigned};
        let mut output = String::new();
        output.push_str("💼 Paper Trading Portfolio\n");
        output.push_str("─────────────────────────────────────────\n");
        output.push_str(&format!(
            "  Starting Balance: {}\n",
            format_currency_unsigned(self.starting_balance)
        ));
        output.push_str(&format!(
            "  Cash Available:   {}\n",
            format_currency_unsigned(self.cash)
        ));

        let open = self.open_positions();
        let closed = self.closed_positions();
        let realized = self.total_realized_pnl();

        // Calculate total unrealized P&L
        let mut total_unrealized = 0.0;
        let mut total_position_value = 0.0;
        for trade in &open {
            if let Some(&current_price) = price_map.get(&trade.symbol) {
                let upnl = trade.unrealized_pnl(current_price);
                total_unrealized += upnl;
                total_position_value += trade.quantity * current_price;
            }
        }

        output.push_str(&format!("  Open Positions:   {}\n", open.len()));
        output.push_str(&format!("  Closed Trades:    {}\n", closed.len()));
        output.push_str(&format!(
            "  Realized P&L:     {}\n",
            format_currency(realized)
        ));
        if !open.is_empty() && !price_map.is_empty() {
            output.push_str(&format!(
                "  Unrealized P&L:   {}\n",
                format_currency(total_unrealized)
            ));
            let total_pnl = realized + total_unrealized;
            let total_value = self.cash + total_position_value;
            let total_return =
                ((total_value - self.starting_balance) / self.starting_balance) * 100.0;
            output.push_str(&format!(
                "  Total P&L:        {} ({}{:.2}%)\n",
                format_currency(total_pnl),
                if total_return >= 0.0 { "+" } else { "" },
                total_return,
            ));
            output.push_str(&format!(
                "  Portfolio Value:  {}\n",
                format_currency_unsigned(total_value)
            ));
        }

        if let Some(wr) = self.win_rate() {
            output.push_str(&format!("  Win Rate:         {:.1}%\n", wr));
        }

        // Show open positions with live P&L
        if !open.is_empty() {
            output.push_str("─────────────────────────────────────────\n");
            output.push_str("  📈 Open Positions:\n");
            for trade in &open {
                let pnl_info = if let Some(&current_price) = price_map.get(&trade.symbol) {
                    let upnl = trade.unrealized_pnl(current_price);
                    let pnl_pct = trade.pnl_pct(current_price);
                    let emoji = if upnl >= 0.0 { "🟢" } else { "🔴" };
                    format!(
                        " → {} {} {} ({}{:.2}%)",
                        format_currency_unsigned(current_price),
                        emoji,
                        format_currency(upnl),
                        if pnl_pct >= 0.0 { "+" } else { "" },
                        pnl_pct,
                    )
                } else {
                    String::new()
                };

                output.push_str(&format!(
                    "    #{} {} {} x{:.4} @ {}{}\n",
                    trade.id,
                    trade.side.to_uppercase(),
                    trade.symbol,
                    trade.quantity,
                    format_currency_unsigned(trade.entry_price),
                    pnl_info,
                ));
                // Show SL/TP if set
                let mut levels = Vec::new();
                if let Some(sl) = trade.stop_loss {
                    levels.push(format!("SL: {}", format_currency_unsigned(sl)));
                }
                if let Some(tp) = trade.take_profit {
                    levels.push(format!("TP: {}", format_currency_unsigned(tp)));
                }
                if !levels.is_empty() {
                    output.push_str(&format!("        🎯 {}\n", levels.join(" | ")));
                }
                if !trade.reasoning.is_empty() {
                    let reason = if trade.reasoning.len() > 60 {
                        format!("{}...", &trade.reasoning[..57])
                    } else {
                        trade.reasoning.clone()
                    };
                    output.push_str(&format!("        Reason: {}\n", reason));
                }
            }
        }

        // Show recent closed trades (last 5)
        if !closed.is_empty() {
            output.push_str("─────────────────────────────────────────\n");
            output.push_str("  📊 Recent Closed Trades:\n");
            for trade in closed.iter().rev().take(5) {
                let pnl = trade.realized_pnl.unwrap_or(0.0);
                let pnl_emoji = if pnl >= 0.0 { "🟢" } else { "🔴" };
                output.push_str(&format!(
                    "    #{} {} {} x{:.4} @ {} → {} {} {}\n",
                    trade.id,
                    trade.side.to_uppercase(),
                    trade.symbol,
                    trade.quantity,
                    format_currency_unsigned(trade.entry_price),
                    format_currency_unsigned(trade.exit_price.unwrap_or(0.0)),
                    pnl_emoji,
                    format_currency(pnl),
                ));
            }
        }

        output.push_str("─────────────────────────────────────────\n");
        output.push_str("  ⚠️  Paper trading only — no real money at risk.\n");

        output
    }

    /// Get portfolio summary as formatted text.
    pub fn summary(&self) -> String {
        use super::format::{format_currency, format_currency_unsigned};
        let mut output = String::new();
        output.push_str("💼 Paper Trading Portfolio\n");
        output.push_str("─────────────────────────────────────────\n");
        output.push_str(&format!(
            "  Starting Balance: {}\n",
            format_currency_unsigned(self.starting_balance)
        ));
        output.push_str(&format!(
            "  Cash Available:   {}\n",
            format_currency_unsigned(self.cash)
        ));

        let open = self.open_positions();
        let closed = self.closed_positions();
        let realized = self.total_realized_pnl();

        output.push_str(&format!("  Open Positions:   {}\n", open.len()));
        output.push_str(&format!("  Closed Trades:    {}\n", closed.len()));
        output.push_str(&format!(
            "  Realized P&L:     {}\n",
            format_currency(realized)
        ));

        if let Some(wr) = self.win_rate() {
            output.push_str(&format!("  Win Rate:         {:.1}%\n", wr));
        }

        // Show open positions
        if !open.is_empty() {
            output.push_str("─────────────────────────────────────────\n");
            output.push_str("  📈 Open Positions:\n");
            for trade in &open {
                output.push_str(&format!(
                    "    #{} {} {} x{:.4} @ {} ({})\n",
                    trade.id,
                    trade.side.to_uppercase(),
                    trade.symbol,
                    trade.quantity,
                    format_currency_unsigned(trade.entry_price),
                    trade.entry_time,
                ));
                // Show SL/TP if set
                let mut levels = Vec::new();
                if let Some(sl) = trade.stop_loss {
                    levels.push(format!("SL: {}", format_currency_unsigned(sl)));
                }
                if let Some(tp) = trade.take_profit {
                    levels.push(format!("TP: {}", format_currency_unsigned(tp)));
                }
                if !levels.is_empty() {
                    output.push_str(&format!("        🎯 {}\n", levels.join(" | ")));
                }
                if !trade.reasoning.is_empty() {
                    let reason = if trade.reasoning.len() > 60 {
                        format!("{}...", &trade.reasoning[..57])
                    } else {
                        trade.reasoning.clone()
                    };
                    output.push_str(&format!("        Reason: {}\n", reason));
                }
            }
        }

        // Show recent closed trades (last 5)
        if !closed.is_empty() {
            output.push_str("─────────────────────────────────────────\n");
            output.push_str("  📊 Recent Closed Trades:\n");
            for trade in closed.iter().rev().take(5) {
                let pnl = trade.realized_pnl.unwrap_or(0.0);
                let pnl_emoji = if pnl >= 0.0 { "🟢" } else { "🔴" };
                output.push_str(&format!(
                    "    #{} {} {} x{:.4} @ {} → {} {} {}\n",
                    trade.id,
                    trade.side.to_uppercase(),
                    trade.symbol,
                    trade.quantity,
                    format_currency_unsigned(trade.entry_price),
                    format_currency_unsigned(trade.exit_price.unwrap_or(0.0)),
                    pnl_emoji,
                    format_currency(pnl),
                ));
            }
        }

        output.push_str("─────────────────────────────────────────\n");
        output.push_str("  ⚠️  Paper trading only — no real money at risk.\n");

        output
    }

    /// Generate a performance dashboard with stats by symbol, time analysis, and streaks.
    /// This helps traders understand which assets they trade best and identify patterns.
    pub fn performance_report(&self) -> String {
        use super::format::format_currency;
        let mut output = String::new();
        let closed = self.closed_positions();

        output.push_str("📊 Performance Dashboard\n");
        output.push_str("═════════════════════════════════════════\n");

        if closed.is_empty() {
            output.push_str("  No closed trades yet. Start trading with /pf buy <symbol> <qty>!\n");
            output.push_str("═════════════════════════════════════════\n");
            return output;
        }

        // Overall stats
        let total_pnl = self.total_realized_pnl();
        let win_rate = self.win_rate().unwrap_or(0.0);
        let wins = closed
            .iter()
            .filter(|t| t.realized_pnl.unwrap_or(0.0) > 0.0)
            .count();
        let losses = closed
            .iter()
            .filter(|t| t.realized_pnl.unwrap_or(0.0) < 0.0)
            .count();

        output.push_str(&format!("  Total Closed:   {}\n", closed.len()));
        output.push_str(&format!(
            "  Total P&L:      {}\n",
            format_currency(total_pnl)
        ));
        output.push_str(&format!(
            "  Win Rate:       {:.1}% ({} W / {} L)\n",
            win_rate, wins, losses
        ));

        // Calculate max consecutive wins/losses (streaks)
        let mut max_win_streak = 0u32;
        let mut max_loss_streak = 0u32;
        let mut current_win_streak = 0u32;
        let mut current_loss_streak = 0u32;

        for trade in &closed {
            let pnl = trade.realized_pnl.unwrap_or(0.0);
            if pnl > 0.0 {
                current_win_streak += 1;
                current_loss_streak = 0;
                max_win_streak = max_win_streak.max(current_win_streak);
            } else {
                current_loss_streak += 1;
                current_win_streak = 0;
                max_loss_streak = max_loss_streak.max(current_loss_streak);
            }
        }

        output.push_str(&format!(
            "  Best Streak:    {} consecutive wins 🔥\n",
            max_win_streak
        ));
        output.push_str(&format!(
            "  Worst Streak:   {} consecutive losses 💀\n",
            max_loss_streak
        ));

        // Stats by symbol
        let mut symbol_stats: std::collections::HashMap<String, SymbolStats> =
            std::collections::HashMap::new();
        for trade in &closed {
            let pnl = trade.realized_pnl.unwrap_or(0.0);
            let stats = symbol_stats
                .entry(trade.symbol.clone())
                .or_insert_with(|| SymbolStats {
                    total_trades: 0,
                    wins: 0,
                    losses: 0,
                    total_pnl: 0.0,
                    best_pnl: f64::NEG_INFINITY,
                    worst_pnl: f64::INFINITY,
                });
            stats.total_trades += 1;
            stats.total_pnl += pnl;
            if pnl > 0.0 {
                stats.wins += 1;
            } else {
                stats.losses += 1;
            }
            if pnl > stats.best_pnl {
                stats.best_pnl = pnl;
            }
            if pnl < stats.worst_pnl {
                stats.worst_pnl = pnl;
            }
        }

        if !symbol_stats.is_empty() {
            output.push_str("─────────────────────────────────────────\n");
            output.push_str("  📈 Performance by Symbol:\n");

            // Sort by total PnL descending
            let mut sorted: Vec<_> = symbol_stats.iter().collect();
            sorted.sort_by(|a, b| {
                b.1.total_pnl
                    .partial_cmp(&a.1.total_pnl)
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            for (symbol, stats) in &sorted {
                let wr = if stats.total_trades > 0 {
                    (stats.wins as f64 / stats.total_trades as f64) * 100.0
                } else {
                    0.0
                };
                let pnl_emoji = if stats.total_pnl >= 0.0 {
                    "🟢"
                } else {
                    "🔴"
                };
                output.push_str(&format!(
                    "    {} {:<12} {} | {} trades | {:.0}% WR | Best: {} | Worst: {}\n",
                    pnl_emoji,
                    symbol,
                    format_currency(stats.total_pnl),
                    stats.total_trades,
                    wr,
                    format_currency(stats.best_pnl),
                    format_currency(stats.worst_pnl),
                ));
            }
        }

        // Confidence analysis — are higher confidence trades more profitable?
        let high_conf: Vec<&&PaperTrade> = closed.iter().filter(|t| t.confidence >= 7).collect();
        let low_conf: Vec<&&PaperTrade> = closed.iter().filter(|t| t.confidence <= 4).collect();

        if !high_conf.is_empty() || !low_conf.is_empty() {
            output.push_str("─────────────────────────────────────────\n");
            output.push_str("  🎯 Confidence Calibration:\n");

            if !high_conf.is_empty() {
                let high_wins = high_conf
                    .iter()
                    .filter(|t| t.realized_pnl.unwrap_or(0.0) > 0.0)
                    .count();
                let high_wr = (high_wins as f64 / high_conf.len() as f64) * 100.0;
                let high_pnl: f64 = high_conf
                    .iter()
                    .map(|t| t.realized_pnl.unwrap_or(0.0))
                    .sum();
                output.push_str(&format!(
                    "    High confidence (7-10): {} trades, {:.0}% WR, {}\n",
                    high_conf.len(),
                    high_wr,
                    format_currency(high_pnl),
                ));
            }

            if !low_conf.is_empty() {
                let low_wins = low_conf
                    .iter()
                    .filter(|t| t.realized_pnl.unwrap_or(0.0) > 0.0)
                    .count();
                let low_wr = (low_wins as f64 / low_conf.len() as f64) * 100.0;
                let low_pnl: f64 = low_conf.iter().map(|t| t.realized_pnl.unwrap_or(0.0)).sum();
                output.push_str(&format!(
                    "    Low confidence (1-4):  {} trades, {:.0}% WR, {}\n",
                    low_conf.len(),
                    low_wr,
                    format_currency(low_pnl),
                ));
            }

            if high_conf.len() >= 3 && low_conf.len() >= 3 {
                let high_wins = high_conf
                    .iter()
                    .filter(|t| t.realized_pnl.unwrap_or(0.0) > 0.0)
                    .count();
                let low_wins = low_conf
                    .iter()
                    .filter(|t| t.realized_pnl.unwrap_or(0.0) > 0.0)
                    .count();
                let high_wr = (high_wins as f64 / high_conf.len() as f64) * 100.0;
                let low_wr = (low_wins as f64 / low_conf.len() as f64) * 100.0;
                if high_wr > low_wr + 10.0 {
                    output.push_str(
                        "    ✅ Your confidence correlates with outcomes — trust your instincts!\n",
                    );
                } else if low_wr > high_wr + 10.0 {
                    output.push_str("    ⚠️  You trade BETTER when less confident — overconfidence may be an issue.\n");
                } else {
                    output.push_str(
                        "    💡 Confidence doesn't predict outcomes much yet. Keep tracking.\n",
                    );
                }
            }
        }

        // Edge analysis: average risk/reward
        let winning_trades: Vec<f64> = closed
            .iter()
            .filter(|t| t.realized_pnl.unwrap_or(0.0) > 0.0)
            .map(|t| t.realized_pnl.unwrap_or(0.0))
            .collect();
        let losing_trades: Vec<f64> = closed
            .iter()
            .filter(|t| t.realized_pnl.unwrap_or(0.0) < 0.0)
            .map(|t| t.realized_pnl.unwrap_or(0.0).abs())
            .collect();

        if !winning_trades.is_empty() && !losing_trades.is_empty() {
            let avg_win = winning_trades.iter().sum::<f64>() / winning_trades.len() as f64;
            let avg_loss = losing_trades.iter().sum::<f64>() / losing_trades.len() as f64;
            let rr_ratio = if avg_loss > 0.0 {
                avg_win / avg_loss
            } else {
                0.0
            };

            output.push_str("─────────────────────────────────────────\n");
            output.push_str("  ⚡ Edge Analysis:\n");
            output.push_str(&format!(
                "    Avg Win:        {}\n",
                format_currency(avg_win)
            ));
            output.push_str(&format!(
                "    Avg Loss:       {}\n",
                format_currency(-avg_loss)
            ));
            output.push_str(&format!("    Risk/Reward:    {:.2}:1\n", rr_ratio));

            // Expected value per trade
            let ev = (win_rate / 100.0) * avg_win - ((100.0 - win_rate) / 100.0) * avg_loss;
            output.push_str(&format!(
                "    Expected Value: {} per trade\n",
                format_currency(ev)
            ));

            if ev > 0.0 && rr_ratio >= 1.5 {
                output.push_str("    ✅ Positive edge — your strategy is working.\n");
            } else if ev > 0.0 {
                output
                    .push_str("    🟡 Slightly positive edge — consider improving risk/reward.\n");
            } else {
                output.push_str("    🔴 Negative edge — review your strategy and entries.\n");
            }
        }

        output.push_str("═════════════════════════════════════════\n");
        output.push_str("  💡 Trade more to get more meaningful statistics.\n");
        output.push_str("  ⚠️  Paper trading only — no real money at risk.\n");
        output
    }
    /// Optional `limit` to show only the most recent N trades (0 = show all).
    pub fn history_report(&self, limit: usize) -> String {
        use super::format::{format_currency, format_currency_unsigned};
        let mut output = String::new();
        let closed = self.closed_positions();
        let open = self.open_positions();

        output.push_str("📜 Trade History\n");
        output.push_str("─────────────────────────────────────────\n");
        output.push_str(&format!(
            "  Total Trades:   {} ({} closed, {} open)\n",
            self.trades.len(),
            closed.len(),
            open.len(),
        ));

        if !closed.is_empty() {
            let realized = self.total_realized_pnl();
            let wins: Vec<&&PaperTrade> = closed
                .iter()
                .filter(|t| t.realized_pnl.unwrap_or(0.0) > 0.0)
                .collect();
            let losses: Vec<&&PaperTrade> = closed
                .iter()
                .filter(|t| t.realized_pnl.unwrap_or(0.0) < 0.0)
                .collect();
            let breakeven: Vec<&&PaperTrade> = closed
                .iter()
                .filter(|t| t.realized_pnl.unwrap_or(0.0) == 0.0)
                .collect();

            let avg_win = if !wins.is_empty() {
                wins.iter()
                    .map(|t| t.realized_pnl.unwrap_or(0.0))
                    .sum::<f64>()
                    / wins.len() as f64
            } else {
                0.0
            };

            let avg_loss = if !losses.is_empty() {
                losses
                    .iter()
                    .map(|t| t.realized_pnl.unwrap_or(0.0))
                    .sum::<f64>()
                    / losses.len() as f64
            } else {
                0.0
            };

            let best_trade = closed.iter().max_by(|a, b| {
                a.realized_pnl
                    .unwrap_or(0.0)
                    .partial_cmp(&b.realized_pnl.unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            let worst_trade = closed.iter().min_by(|a, b| {
                a.realized_pnl
                    .unwrap_or(0.0)
                    .partial_cmp(&b.realized_pnl.unwrap_or(0.0))
                    .unwrap_or(std::cmp::Ordering::Equal)
            });

            // Profit factor: sum of wins / sum of losses (absolute)
            let total_wins: f64 = wins.iter().map(|t| t.realized_pnl.unwrap_or(0.0)).sum();
            let total_losses: f64 = losses
                .iter()
                .map(|t| t.realized_pnl.unwrap_or(0.0).abs())
                .sum();
            let profit_factor = if total_losses > 0.0 {
                total_wins / total_losses
            } else if total_wins > 0.0 {
                f64::INFINITY
            } else {
                0.0
            };

            output.push_str(&format!(
                "  Realized P&L:   {}\n",
                format_currency(realized)
            ));
            if let Some(wr) = self.win_rate() {
                output.push_str(&format!("  Win Rate:       {:.1}%\n", wr));
            }
            output.push_str(&format!(
                "  Wins/Losses/BE: {} / {} / {}\n",
                wins.len(),
                losses.len(),
                breakeven.len(),
            ));
            output.push_str(&format!("  Avg Win:        {}\n", format_currency(avg_win)));
            output.push_str(&format!(
                "  Avg Loss:       {}\n",
                format_currency(avg_loss)
            ));
            if profit_factor.is_finite() {
                output.push_str(&format!("  Profit Factor:  {:.2}\n", profit_factor));
            }

            if let Some(best) = best_trade {
                let pnl = best.realized_pnl.unwrap_or(0.0);
                output.push_str(&format!(
                    "  Best Trade:     #{} {} {} {}\n",
                    best.id,
                    best.side.to_uppercase(),
                    best.symbol,
                    format_currency(pnl),
                ));
            }
            if let Some(worst) = worst_trade {
                let pnl = worst.realized_pnl.unwrap_or(0.0);
                output.push_str(&format!(
                    "  Worst Trade:    #{} {} {} {}\n",
                    worst.id,
                    worst.side.to_uppercase(),
                    worst.symbol,
                    format_currency(pnl),
                ));
            }
        }

        // Show trades in reverse chronological order
        output.push_str("─────────────────────────────────────────\n");

        let trades_to_show: Vec<&PaperTrade> = if limit > 0 {
            self.trades.iter().rev().take(limit).collect()
        } else {
            self.trades.iter().rev().collect()
        };

        if trades_to_show.is_empty() {
            output.push_str("  No trades yet.\n");
        } else {
            if limit > 0 && self.trades.len() > limit {
                output.push_str(&format!(
                    "  Showing {} of {} trades:\n",
                    limit,
                    self.trades.len()
                ));
            }
            for trade in &trades_to_show {
                let status = if trade.is_open() {
                    "📈 OPEN".to_string()
                } else {
                    let pnl = trade.realized_pnl.unwrap_or(0.0);
                    let emoji = if pnl >= 0.0 { "🟢" } else { "🔴" };
                    format!("{} {}", emoji, format_currency(pnl))
                };

                let exit_info = if let Some(exit) = trade.exit_price {
                    format!(" → {}", format_currency_unsigned(exit))
                } else {
                    String::new()
                };

                output.push_str(&format!(
                    "  #{:<3} {} {:<10} x{:.4} @ {}{} {}\n",
                    trade.id,
                    trade.side.to_uppercase(),
                    trade.symbol,
                    trade.quantity,
                    format_currency_unsigned(trade.entry_price),
                    exit_info,
                    status,
                ));
                if !trade.reasoning.is_empty() {
                    let reason = if trade.reasoning.len() > 50 {
                        format!("{}...", &trade.reasoning[..47])
                    } else {
                        trade.reasoning.clone()
                    };
                    output.push_str(&format!("       {}\n", reason));
                }
            }
        }

        output.push_str("─────────────────────────────────────────\n");
        output
    }

    /// Generate an equity curve showing portfolio value at each trade event.
    ///
    /// Returns a list of (label, value) points that can be rendered as an ASCII chart.
    /// Each point represents either a trade open or close event.
    pub fn equity_curve(&self) -> Vec<(String, f64)> {

        // Collect all trade events sorted chronologically
        let mut events: Vec<(String, u32, &str)> = Vec::new(); // (timestamp, trade_id, "open"/"close")

        for trade in &self.trades {
            events.push((trade.entry_time.clone(), trade.id, "open"));
            if let Some(ref exit_time) = trade.exit_time {
                events.push((exit_time.clone(), trade.id, "close"));
            }
        }
        events.sort_by(|a, b| a.0.cmp(&b.0));

        if events.is_empty() {
            return vec![("Start".to_string(), self.starting_balance)];
        }

        // Replay events to compute equity at each point
        let mut curve: Vec<(String, f64)> = Vec::new();
        curve.push(("Start".to_string(), self.starting_balance));

        let mut cash = self.starting_balance;
        let mut open_positions: std::collections::HashMap<u32, (f64, f64, String)> =
            std::collections::HashMap::new(); // id -> (qty, entry_price, side)

        for (timestamp, trade_id, event_type) in &events {
            if let Some(trade) = self.trades.iter().find(|t| t.id == *trade_id) {
                match *event_type {
                    "open" => {
                        let cost = trade.quantity * trade.entry_price;
                        if trade.side == "buy" {
                            cash -= cost;
                        }
                        open_positions.insert(
                            trade.id,
                            (trade.quantity, trade.entry_price, trade.side.clone()),
                        );
                    }
                    "close" => {
                        let exit_price = trade.exit_price.unwrap_or(trade.entry_price);
                        if trade.side == "buy" {
                            cash += trade.quantity * exit_price;
                        } else {
                            let pnl = trade.quantity * (trade.entry_price - exit_price);
                            cash += pnl;
                        }
                        open_positions.remove(&trade.id);
                    }
                    _ => {}
                }

                // Compute total portfolio value (cash + open position values at entry prices)
                // We use entry prices since we don't have historical market prices
                let open_value: f64 = open_positions
                    .values()
                    .map(|(qty, price, _)| qty * price)
                    .sum();
                let total = cash + open_value;

                // Label: use short date from timestamp
                let label = if timestamp.len() >= 10 {
                    timestamp[5..10].to_string() // "MM-DD"
                } else {
                    timestamp.clone()
                };
                curve.push((label, total));
            }
        }

        curve
    }

    /// Render the equity curve as an ASCII chart for terminal display.
    pub fn equity_chart(&self) -> String {
        use super::format::format_currency_unsigned;

        let curve = self.equity_curve();
        let mut output = String::new();

        output.push_str("📈 Equity Curve\n");
        output.push_str("═════════════════════════════════════════\n");

        if curve.len() <= 1 {
            output.push_str("  No trade events to chart yet.\n");
            output.push_str("  Start trading with: /pf buy <symbol> <qty>\n");
            output.push_str("═════════════════════════════════════════\n");
            return output;
        }

        let values: Vec<f64> = curve.iter().map(|(_, v)| *v).collect();
        let min_val = values
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min);
        let max_val = values
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);

        let current = *values.last().unwrap_or(&self.starting_balance);
        let total_return = ((current - self.starting_balance) / self.starting_balance) * 100.0;

        output.push_str(&format!(
            "  Start: {}  →  Now: {} ({}{:.2}%)\n",
            format_currency_unsigned(self.starting_balance),
            format_currency_unsigned(current),
            if total_return >= 0.0 { "+" } else { "" },
            total_return,
        ));
        output.push_str(&format!(
            "  Peak: {}  |  Trough: {}\n",
            format_currency_unsigned(max_val),
            format_currency_unsigned(min_val),
        ));

        // Max drawdown
        let mut peak = f64::NEG_INFINITY;
        let mut max_dd = 0.0f64;
        for &v in &values {
            if v > peak {
                peak = v;
            }
            let dd = (peak - v) / peak * 100.0;
            if dd > max_dd {
                max_dd = dd;
            }
        }
        output.push_str(&format!("  Max Drawdown: {:.2}%\n", max_dd));

        output.push_str("─────────────────────────────────────────\n");

        // Render ASCII chart (40 chars wide, 12 rows tall)
        let chart_width = 50;
        let chart_height = 12;
        let range = max_val - min_val;

        if range < 0.01 {
            // Flat line
            output.push_str("  (Portfolio value is flat — chart not meaningful)\n");
        } else {
            // Resample data to fit chart width
            let data_len = values.len();
            let step = if data_len > chart_width {
                data_len as f64 / chart_width as f64
            } else {
                1.0
            };
            let resampled: Vec<f64> = (0..chart_width.min(data_len))
                .map(|i| {
                    let idx = (i as f64 * step) as usize;
                    values[idx.min(data_len - 1)]
                })
                .collect();

            // Render chart rows (top to bottom)
            for row in (0..chart_height).rev() {
                let threshold = min_val + (range * row as f64 / (chart_height - 1) as f64);

                // Y-axis label (only on first, middle, and last rows)
                let label = if row == chart_height - 1 {
                    format!("{:>10}", format_currency_unsigned(max_val))
                } else if row == 0 {
                    format!("{:>10}", format_currency_unsigned(min_val))
                } else if row == chart_height / 2 {
                    let mid = (min_val + max_val) / 2.0;
                    format!("{:>10}", format_currency_unsigned(mid))
                } else {
                    "          ".to_string()
                };

                let mut line = format!("  {} │", label);
                for &val in &resampled {
                    if val >= threshold {
                        // Color based on whether above or below starting balance
                        if val >= self.starting_balance {
                            line.push('█');
                        } else {
                            line.push('▓');
                        }
                    } else {
                        line.push(' ');
                    }
                }
                output.push_str(&line);
                output.push('\n');
            }

            // X-axis
            output.push_str(&format!("  {} └{}\n", "          ", "─".repeat(resampled.len())));

            // X-axis labels (start, middle, end)
            let start_label = &curve.first().map(|(l, _)| l.as_str()).unwrap_or("");
            let end_label = &curve.last().map(|(l, _)| l.as_str()).unwrap_or("");
            let padding = if resampled.len() > 10 {
                resampled.len() - start_label.len() - end_label.len()
            } else {
                2
            };
            output.push_str(&format!(
                "  {}  {}{}{}\n",
                "          ",
                start_label,
                " ".repeat(padding.max(1)),
                end_label,
            ));
        }

        // Trade event summary
        let trade_count = self.trades.len();
        let closed_count = self.closed_positions().len();
        let open_count = self.open_positions().len();

        output.push_str("─────────────────────────────────────────\n");
        output.push_str(&format!(
            "  {} trade events ({} closed, {} open)\n",
            trade_count, closed_count, open_count,
        ));

        // Show sparkline version too for compact view
        let sparkline_chars = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];
        let sparkline: String = values
            .iter()
            .map(|&v| {
                let normalized = if range > 0.0 {
                    ((v - min_val) / range * 7.0) as usize
                } else {
                    4
                };
                sparkline_chars[normalized.min(7)]
            })
            .collect();
        output.push_str(&format!("  Sparkline: {}\n", sparkline));

        output.push_str("═════════════════════════════════════════\n");
        output.push_str("  ⚠️  Paper trading only — no real money at risk.\n");

        output
    }
}

/// Get current timestamp as ISO 8601 string.
fn current_timestamp() -> String {
    super::format::current_timestamp()
}

/// Export all trades as CSV for spreadsheet analysis.
///
/// Columns: ID, Symbol, Side, Quantity, Entry Price, Exit Price, P&L, P&L %, Confidence,
/// Stop Loss, Take Profit, Entry Time, Exit Time, Reasoning, Status
pub fn export_trades_csv(portfolio: &Portfolio) -> String {
    let mut csv = String::from(
        "ID,Symbol,Side,Quantity,Entry Price,Exit Price,P&L,P&L %,Confidence,Stop Loss,Take Profit,Entry Time,Exit Time,Reasoning,Status\n"
    );

    for trade in &portfolio.trades {
        let exit_price = trade
            .exit_price
            .map_or(String::new(), |p| format!("{:.2}", p));
        let pnl = trade
            .realized_pnl
            .map_or(String::new(), |p| format!("{:.2}", p));
        let pnl_pct = if let Some(exit) = trade.exit_price {
            format!("{:.2}", trade.pnl_pct(exit))
        } else {
            String::new()
        };
        let sl = trade
            .stop_loss
            .map_or(String::new(), |p| format!("{:.2}", p));
        let tp = trade
            .take_profit
            .map_or(String::new(), |p| format!("{:.2}", p));
        let exit_time = trade.exit_time.as_deref().unwrap_or("");
        let status = if trade.is_open() { "open" } else { "closed" };
        // Escape reasoning for CSV (double-quote any quotes, wrap in quotes)
        let reasoning = trade.reasoning.replace('"', "\"\"");

        csv.push_str(&format!(
            "{},{},{},{:.6},{:.2},{},{},{},{},{},{},{},{},\"{}\",{}\n",
            trade.id,
            trade.symbol,
            trade.side,
            trade.quantity,
            trade.entry_price,
            exit_price,
            pnl,
            pnl_pct,
            trade.confidence,
            sl,
            tp,
            trade.entry_time,
            exit_time,
            reasoning,
            status,
        ));
    }

    csv
}

/// Compute follow-up analysis for a closed trade.
/// Returns (since_exit_pct, hypothetical_pnl, diff_from_actual, verdict).
/// `current_price` is the live price of the asset now.
pub fn compute_trade_followup(
    trade: &PaperTrade,
    current_price: f64,
) -> (f64, f64, f64, &'static str) {
    let exit_price = trade.exit_price.unwrap_or(trade.entry_price);
    let pnl = trade.realized_pnl.unwrap_or(0.0);

    let since_exit_pct = if exit_price > 0.0 {
        ((current_price - exit_price) / exit_price) * 100.0
    } else {
        0.0
    };

    let hypothetical_pnl = if trade.side == "buy" {
        (current_price - trade.entry_price) * trade.quantity
    } else {
        (trade.entry_price - current_price) * trade.quantity
    };

    let diff = hypothetical_pnl - pnl;

    let verdict = if trade.side == "buy" {
        if since_exit_pct > 5.0 {
            "exited_early_significant"
        } else if since_exit_pct > 2.0 {
            "exited_early_minor"
        } else if since_exit_pct < -5.0 {
            "good_exit_significant"
        } else if since_exit_pct < -2.0 {
            "good_exit_minor"
        } else {
            "neutral"
        }
    } else {
        if since_exit_pct < -5.0 {
            "covered_early_significant"
        } else if since_exit_pct < -2.0 {
            "covered_early_minor"
        } else if since_exit_pct > 5.0 {
            "good_cover_significant"
        } else if since_exit_pct > 2.0 {
            "good_cover_minor"
        } else {
            "neutral"
        }
    };

    (since_exit_pct, hypothetical_pnl, diff, verdict)
}

/// Log a trade entry to TRADES.md for accountability tracking.
/// This keeps TRADES.md in sync with the portfolio state.
pub fn log_trade_to_journal(trade: &PaperTrade, action: &str) -> Result<(), String> {
    let trades_file = "TRADES.md";
    let content = std::fs::read_to_string(trades_file).unwrap_or_default();

    // Find the trade log section
    let entry = match action {
        "open" => {
            format!(
                "\n### Trade #{} — {} {} (Paper)\n\
                 - **Type:** paper\n\
                 - **Action:** {}\n\
                 - **Symbol:** {}\n\
                 - **Entry price:** ${:.2}\n\
                 - **Exit price:** open\n\
                 - **Size:** {:.6} units\n\
                 - **P&L:** open\n\
                 - **My reasoning:** {}\n\
                 - **Confidence at entry:** {}/10\n\
                 - **Opened:** {}\n\n",
                trade.id,
                trade.symbol.to_uppercase(),
                trade.side.to_uppercase(),
                trade.side,
                trade.symbol,
                trade.entry_price,
                trade.quantity,
                if trade.reasoning.is_empty() {
                    "(no reason given)"
                } else {
                    &trade.reasoning
                },
                trade.confidence,
                trade.entry_time,
            )
        }
        "close" => {
            let pnl = trade.realized_pnl.unwrap_or(0.0);
            let exit_price = trade.exit_price.unwrap_or(0.0);
            let pnl_pct = trade.pnl_pct(exit_price);
            format!(
                "\n### Trade #{} — {} {} CLOSED\n\
                 - **Exit price:** ${:.2}\n\
                 - **P&L:** {}{:.2} ({:.2}%)\n\
                 - **Closed:** {}\n\n",
                trade.id,
                trade.symbol.to_uppercase(),
                trade.side.to_uppercase(),
                exit_price,
                if pnl >= 0.0 { "+$" } else { "-$" },
                pnl.abs(),
                pnl_pct,
                trade.exit_time.as_deref().unwrap_or("unknown"),
            )
        }
        _ => return Ok(()),
    };

    // Insert after "(No trades yet. Paper trading comes first.)" or at end of Trade Log section
    let new_content = if content.contains("(No trades yet. Paper trading comes first.)") {
        content.replace(
            "(No trades yet. Paper trading comes first.)",
            &format!("{}", entry.trim()),
        )
    } else if let Some(pos) = content.find("## Recurring Mistakes") {
        let (before, after) = content.split_at(pos);
        format!("{}{}\n{}", before, entry, after)
    } else {
        format!("{}\n{}", content, entry)
    };

    std::fs::write(trades_file, new_content)
        .map_err(|e| format!("Failed to write TRADES.md: {}", e))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_portfolio() {
        let p = Portfolio::new();
        assert_eq!(p.starting_balance, 100_000.0);
        assert_eq!(p.cash, 100_000.0);
        assert!(p.trades.is_empty());
        assert_eq!(p.next_id, 1);
    }

    #[test]
    fn test_open_trade_buy() {
        let mut p = Portfolio::new();
        let id = p
            .open_trade("bitcoin", "buy", 1.0, 87000.0, "BTC looks bullish", 7)
            .unwrap();
        assert_eq!(id, 1);
        assert_eq!(p.cash, 100_000.0 - 87_000.0);
        assert_eq!(p.trades.len(), 1);
        assert!(p.trades[0].is_open());
    }

    #[test]
    fn test_open_trade_insufficient_cash() {
        let mut p = Portfolio::new();
        let result = p.open_trade("bitcoin", "buy", 2.0, 87000.0, "YOLO", 5);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Insufficient cash"));
    }

    #[test]
    fn test_open_trade_invalid_quantity() {
        let mut p = Portfolio::new();
        assert!(p.open_trade("bitcoin", "buy", 0.0, 87000.0, "", 5).is_err());
        assert!(p
            .open_trade("bitcoin", "buy", -1.0, 87000.0, "", 5)
            .is_err());
    }

    #[test]
    fn test_open_trade_invalid_side() {
        let mut p = Portfolio::new();
        assert!(p
            .open_trade("bitcoin", "hodl", 1.0, 87000.0, "", 5)
            .is_err());
    }

    #[test]
    fn test_close_trade() {
        let mut p = Portfolio::new();
        let id = p
            .open_trade("AAPL", "buy", 10.0, 200.0, "Earnings play", 6)
            .unwrap();
        assert_eq!(p.cash, 100_000.0 - 2000.0);

        let pnl = p.close_trade(id, 220.0).unwrap();
        assert_eq!(pnl, 200.0); // 10 * (220-200) = 200
        assert_eq!(p.cash, 100_000.0 - 2000.0 + 2200.0); // Got back more
        assert!(!p.trades[0].is_open());
    }

    #[test]
    fn test_close_trade_loss() {
        let mut p = Portfolio::new();
        let id = p
            .open_trade("AAPL", "buy", 10.0, 200.0, "Bad timing", 4)
            .unwrap();
        let pnl = p.close_trade(id, 180.0).unwrap();
        assert_eq!(pnl, -200.0); // 10 * (180-200) = -200
    }

    #[test]
    fn test_close_nonexistent_trade() {
        let mut p = Portfolio::new();
        assert!(p.close_trade(999, 100.0).is_err());
    }

    #[test]
    fn test_unrealized_pnl() {
        let mut p = Portfolio::new();
        p.open_trade("bitcoin", "buy", 0.5, 80000.0, "DCA", 5)
            .unwrap();
        let trade = &p.trades[0];
        // If BTC goes to 90000, unrealized = 0.5 * (90000 - 80000) = 5000
        assert_eq!(trade.unrealized_pnl(90000.0), 5000.0);
        // If BTC drops to 70000, unrealized = 0.5 * (70000 - 80000) = -5000
        assert_eq!(trade.unrealized_pnl(70000.0), -5000.0);
    }

    #[test]
    fn test_sell_trade_pnl() {
        let mut p = Portfolio::new();
        p.open_trade("bitcoin", "sell", 0.5, 90000.0, "Shorting BTC", 5)
            .unwrap();
        let trade = &p.trades[0];
        // Short: sell at 90k, price drops to 80k → profit
        assert_eq!(trade.unrealized_pnl(80000.0), 5000.0);
        // Short: sell at 90k, price rises to 100k → loss
        assert_eq!(trade.unrealized_pnl(100000.0), -5000.0);
    }

    #[test]
    fn test_win_rate() {
        let mut p = Portfolio::new();
        // Win
        let id1 = p.open_trade("AAPL", "buy", 1.0, 100.0, "W", 5).unwrap();
        p.close_trade(id1, 110.0).unwrap();
        // Win
        let id2 = p.open_trade("MSFT", "buy", 1.0, 100.0, "W", 5).unwrap();
        p.close_trade(id2, 105.0).unwrap();
        // Loss
        let id3 = p.open_trade("TSLA", "buy", 1.0, 100.0, "L", 5).unwrap();
        p.close_trade(id3, 90.0).unwrap();

        let wr = p.win_rate().unwrap();
        assert!(
            (wr - 66.67).abs() < 0.1,
            "Win rate should be ~66.7%, got {}",
            wr
        );
    }

    #[test]
    fn test_total_realized_pnl() {
        let mut p = Portfolio::new();
        let id1 = p.open_trade("AAPL", "buy", 10.0, 100.0, "", 5).unwrap();
        p.close_trade(id1, 110.0).unwrap(); // +100
        let id2 = p.open_trade("MSFT", "buy", 5.0, 200.0, "", 5).unwrap();
        p.close_trade(id2, 190.0).unwrap(); // -50
        assert_eq!(p.total_realized_pnl(), 50.0); // net +50
    }

    #[test]
    fn test_portfolio_summary_not_empty() {
        let p = Portfolio::new();
        let summary = p.summary();
        assert!(summary.contains("Paper Trading Portfolio"));
        assert!(summary.contains("$100,000"));
    }

    #[test]
    fn test_open_and_closed_positions() {
        let mut p = Portfolio::new();
        p.open_trade("AAPL", "buy", 1.0, 100.0, "", 5).unwrap();
        let id2 = p.open_trade("MSFT", "buy", 1.0, 200.0, "", 5).unwrap();
        p.close_trade(id2, 210.0).unwrap();

        assert_eq!(p.open_positions().len(), 1);
        assert_eq!(p.closed_positions().len(), 1);
    }

    #[test]
    fn test_pnl_pct() {
        let mut p = Portfolio::new();
        p.open_trade("AAPL", "buy", 10.0, 100.0, "", 5).unwrap();
        let trade = &p.trades[0];
        // 10% gain
        let pct = trade.pnl_pct(110.0);
        assert!((pct - 10.0).abs() < 0.01);
        // 5% loss
        let pct = trade.pnl_pct(95.0);
        assert!((pct - (-5.0)).abs() < 0.01);
    }

    #[test]
    fn test_current_timestamp_format() {
        let ts = current_timestamp();
        assert!(ts.contains('T'));
        assert!(ts.ends_with('Z'));
        // Should be a reasonable year
        assert!(ts.starts_with("20"));
    }

    #[test]
    fn test_confidence_validation() {
        let mut p = Portfolio::new();
        assert!(p.open_trade("BTC", "buy", 1.0, 100.0, "", 11).is_err());
    }

    #[test]
    fn test_multiple_trades_same_symbol() {
        let mut p = Portfolio::new();
        p.open_trade("bitcoin", "buy", 0.1, 80000.0, "First buy", 5)
            .unwrap();
        p.open_trade("bitcoin", "buy", 0.2, 82000.0, "Adding", 6)
            .unwrap();
        assert_eq!(p.open_positions().len(), 2);
        let total_btc: f64 = p
            .open_positions()
            .iter()
            .filter(|t| t.symbol == "bitcoin")
            .map(|t| t.quantity)
            .sum();
        assert!((total_btc - 0.3).abs() < 0.001);
    }

    #[test]
    fn test_summary_with_prices() {
        let mut p = Portfolio::new();
        p.open_trade("bitcoin", "buy", 0.5, 80000.0, "BTC dip buy", 7)
            .unwrap();
        p.open_trade("AAPL", "buy", 10.0, 200.0, "Apple earnings", 6)
            .unwrap();

        let mut prices = std::collections::HashMap::new();
        prices.insert("bitcoin".to_string(), 90000.0);
        prices.insert("AAPL".to_string(), 210.0);

        let summary = p.summary_with_prices(&prices);
        assert!(summary.contains("Paper Trading Portfolio"));
        assert!(summary.contains("Unrealized P&L"));
        assert!(summary.contains("Total P&L"));
        assert!(summary.contains("Portfolio Value"));
        // BTC unrealized: 0.5 * (90000-80000) = 5000
        // AAPL unrealized: 10 * (210-200) = 100
        // Total unrealized: 5100
        assert!(
            summary.contains("+$5,100.00"),
            "Summary should contain formatted P&L, got: {}",
            summary
        );
    }

    #[test]
    fn test_summary_with_prices_empty() {
        let p = Portfolio::new();
        let prices = std::collections::HashMap::new();
        let summary = p.summary_with_prices(&prices);
        assert!(summary.contains("Paper Trading Portfolio"));
        // No open positions means no unrealized P&L section
        assert!(!summary.contains("Unrealized"));
    }

    #[test]
    fn test_summary_with_prices_loss() {
        let mut p = Portfolio::new();
        p.open_trade("bitcoin", "buy", 1.0, 90000.0, "Top signal", 5)
            .unwrap();

        let mut prices = std::collections::HashMap::new();
        prices.insert("bitcoin".to_string(), 85000.0);

        let summary = p.summary_with_prices(&prices);
        // Should show negative unrealized P&L
        assert!(
            summary.contains("-$5,000.00"),
            "Summary should contain formatted loss, got: {}",
            summary
        );
    }

    #[test]
    fn test_open_trade_with_stop_loss() {
        let mut p = Portfolio::new();
        let id = p
            .open_trade_with_levels(
                "bitcoin",
                "buy",
                0.5,
                90000.0,
                "SL test",
                5,
                Some(85000.0),
                None,
            )
            .unwrap();
        let trade = p.trades.iter().find(|t| t.id == id).unwrap();
        assert_eq!(trade.stop_loss, Some(85000.0));
        assert_eq!(trade.take_profit, None);
    }

    #[test]
    fn test_open_trade_with_take_profit() {
        let mut p = Portfolio::new();
        let id = p
            .open_trade_with_levels(
                "bitcoin",
                "buy",
                0.5,
                90000.0,
                "TP test",
                5,
                None,
                Some(100000.0),
            )
            .unwrap();
        let trade = p.trades.iter().find(|t| t.id == id).unwrap();
        assert_eq!(trade.stop_loss, None);
        assert_eq!(trade.take_profit, Some(100000.0));
    }

    #[test]
    fn test_open_trade_with_both_sl_tp() {
        let mut p = Portfolio::new();
        let id = p
            .open_trade_with_levels(
                "AAPL",
                "buy",
                10.0,
                200.0,
                "Both",
                7,
                Some(190.0),
                Some(220.0),
            )
            .unwrap();
        let trade = p.trades.iter().find(|t| t.id == id).unwrap();
        assert_eq!(trade.stop_loss, Some(190.0));
        assert_eq!(trade.take_profit, Some(220.0));
    }

    #[test]
    fn test_stop_loss_validation_buy() {
        let mut p = Portfolio::new();
        // SL above entry for a buy should fail
        let result = p.open_trade_with_levels("AAPL", "buy", 10.0, 200.0, "", 5, Some(210.0), None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("below entry"));
    }

    #[test]
    fn test_stop_loss_validation_sell() {
        let mut p = Portfolio::new();
        // SL below entry for a short should fail
        let result =
            p.open_trade_with_levels("AAPL", "sell", 10.0, 200.0, "", 5, Some(190.0), None);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("above entry"));
    }

    #[test]
    fn test_take_profit_validation_buy() {
        let mut p = Portfolio::new();
        // TP below entry for a buy should fail
        let result = p.open_trade_with_levels("AAPL", "buy", 10.0, 200.0, "", 5, None, Some(190.0));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("above entry"));
    }

    #[test]
    fn test_check_stop_loss_triggered() {
        let mut p = Portfolio::new();
        p.open_trade_with_levels(
            "bitcoin",
            "buy",
            0.5,
            90000.0,
            "",
            5,
            Some(85000.0),
            Some(100000.0),
        )
        .unwrap();

        let mut prices = std::collections::HashMap::new();
        prices.insert("bitcoin".to_string(), 84000.0); // Below SL

        let triggered = p.check_stop_loss_take_profit(&prices);
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].2, "stop-loss");
    }

    #[test]
    fn test_check_take_profit_triggered() {
        let mut p = Portfolio::new();
        p.open_trade_with_levels(
            "bitcoin",
            "buy",
            0.5,
            90000.0,
            "",
            5,
            Some(85000.0),
            Some(100000.0),
        )
        .unwrap();

        let mut prices = std::collections::HashMap::new();
        prices.insert("bitcoin".to_string(), 101000.0); // Above TP

        let triggered = p.check_stop_loss_take_profit(&prices);
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].2, "take-profit");
    }

    #[test]
    fn test_check_no_trigger() {
        let mut p = Portfolio::new();
        p.open_trade_with_levels(
            "bitcoin",
            "buy",
            0.5,
            90000.0,
            "",
            5,
            Some(85000.0),
            Some(100000.0),
        )
        .unwrap();

        let mut prices = std::collections::HashMap::new();
        prices.insert("bitcoin".to_string(), 92000.0); // Between SL and TP

        let triggered = p.check_stop_loss_take_profit(&prices);
        assert!(triggered.is_empty());
    }

    #[test]
    fn test_check_short_stop_loss() {
        let mut p = Portfolio::new();
        p.open_trade_with_levels(
            "bitcoin",
            "sell",
            0.5,
            90000.0,
            "",
            5,
            Some(95000.0),
            Some(80000.0),
        )
        .unwrap();

        let mut prices = std::collections::HashMap::new();
        prices.insert("bitcoin".to_string(), 96000.0); // Above SL for short

        let triggered = p.check_stop_loss_take_profit(&prices);
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].2, "stop-loss");
    }

    #[test]
    fn test_check_short_take_profit() {
        let mut p = Portfolio::new();
        p.open_trade_with_levels(
            "bitcoin",
            "sell",
            0.5,
            90000.0,
            "",
            5,
            Some(95000.0),
            Some(80000.0),
        )
        .unwrap();

        let mut prices = std::collections::HashMap::new();
        prices.insert("bitcoin".to_string(), 79000.0); // Below TP for short

        let triggered = p.check_stop_loss_take_profit(&prices);
        assert_eq!(triggered.len(), 1);
        assert_eq!(triggered[0].2, "take-profit");
    }

    #[test]
    fn test_history_report_empty() {
        let p = Portfolio::new();
        let report = p.history_report(20);
        assert!(report.contains("Trade History"));
        assert!(report.contains("No trades yet"));
    }

    #[test]
    fn test_history_report_with_trades() {
        let mut p = Portfolio::new();
        let id1 = p
            .open_trade("AAPL", "buy", 10.0, 100.0, "Test trade", 5)
            .unwrap();
        p.close_trade(id1, 110.0).unwrap(); // +100 win
        let id2 = p
            .open_trade("MSFT", "buy", 5.0, 200.0, "Another test", 6)
            .unwrap();
        p.close_trade(id2, 190.0).unwrap(); // -50 loss
                                            // Open trade
        p.open_trade("TSLA", "buy", 2.0, 300.0, "Open position", 7)
            .unwrap();

        let report = p.history_report(0);
        assert!(report.contains("Trade History"));
        assert!(report.contains("2 closed")); // 2 closed, 1 open
        assert!(report.contains("1 open"));
        assert!(report.contains("Win Rate"));
        assert!(report.contains("Avg Win"));
        assert!(report.contains("Profit Factor"));
        assert!(report.contains("AAPL"));
        assert!(report.contains("MSFT"));
        assert!(report.contains("TSLA"));
    }

    #[test]
    fn test_history_report_with_limit() {
        let mut p = Portfolio::new();
        for i in 0..10 {
            p.open_trade("AAPL", "buy", 1.0, 100.0 + i as f64, "", 5)
                .unwrap();
        }
        let report = p.history_report(3);
        assert!(report.contains("Showing 3 of 10"));
    }

    #[test]
    fn test_performance_report_empty() {
        let p = Portfolio::new();
        let report = p.performance_report();
        assert!(report.contains("Performance Dashboard"));
        assert!(report.contains("No closed trades yet"));
    }

    #[test]
    fn test_performance_report_with_trades() {
        let mut p = Portfolio::new();
        // Win on AAPL
        let id1 = p.open_trade("AAPL", "buy", 10.0, 100.0, "W", 8).unwrap();
        p.close_trade(id1, 110.0).unwrap();
        // Win on AAPL
        let id2 = p.open_trade("AAPL", "buy", 10.0, 105.0, "W", 7).unwrap();
        p.close_trade(id2, 115.0).unwrap();
        // Loss on MSFT
        let id3 = p.open_trade("MSFT", "buy", 5.0, 200.0, "L", 3).unwrap();
        p.close_trade(id3, 190.0).unwrap();

        let report = p.performance_report();
        assert!(
            report.contains("Performance Dashboard"),
            "Should have dashboard header"
        );
        assert!(report.contains("AAPL"), "Should show AAPL stats");
        assert!(report.contains("MSFT"), "Should show MSFT stats");
        assert!(
            report.contains("Performance by Symbol"),
            "Should show per-symbol breakdown"
        );
        assert!(report.contains("Streak"), "Should show streaks");
        assert!(
            report.contains("Edge Analysis"),
            "Should show edge analysis"
        );
    }

    #[test]
    fn test_performance_report_confidence_calibration() {
        let mut p = Portfolio::new();
        // High confidence wins
        for i in 0..5 {
            let id = p
                .open_trade("BTC", "buy", 0.1, 80000.0 + i as f64 * 100.0, "HC", 8)
                .unwrap();
            p.close_trade(id, 81000.0 + i as f64 * 100.0).unwrap();
        }
        // Low confidence losses
        for i in 0..5 {
            let id = p
                .open_trade("ETH", "buy", 1.0, 3000.0 + i as f64 * 10.0, "LC", 3)
                .unwrap();
            p.close_trade(id, 2900.0 + i as f64 * 10.0).unwrap();
        }
        let report = p.performance_report();
        assert!(
            report.contains("Confidence Calibration"),
            "Should show confidence analysis"
        );
    }

    #[test]
    fn test_followup_buy_exited_early() {
        let trade = PaperTrade {
            id: 1,
            symbol: "bitcoin".into(),
            side: "buy".into(),
            quantity: 1.0,
            entry_price: 80000.0,
            exit_price: Some(85000.0),
            reasoning: String::new(),
            confidence: 7,
            entry_time: "2026-01-01".into(),
            exit_time: Some("2026-01-10".into()),
            realized_pnl: Some(5000.0),
            stop_loss: None,
            take_profit: None,
            trailing_stop_pct: None,
            highest_price_seen: None,
            lowest_price_seen: None,
        };

        // Price went to 95000 after exit — exited too early
        let (since_exit_pct, hypo_pnl, diff, verdict) = compute_trade_followup(&trade, 95000.0);
        assert!(since_exit_pct > 10.0); // ~11.8% up since exit
        assert_eq!(hypo_pnl, 15000.0); // Would have made $15K if held
        assert_eq!(diff, 10000.0); // Left $10K on table
        assert_eq!(verdict, "exited_early_significant");
    }

    #[test]
    fn test_followup_buy_good_exit() {
        let trade = PaperTrade {
            id: 2,
            symbol: "bitcoin".into(),
            side: "buy".into(),
            quantity: 1.0,
            entry_price: 80000.0,
            exit_price: Some(85000.0),
            reasoning: String::new(),
            confidence: 7,
            entry_time: "2026-01-01".into(),
            exit_time: Some("2026-01-10".into()),
            realized_pnl: Some(5000.0),
            stop_loss: None,
            take_profit: None,
            trailing_stop_pct: None,
            highest_price_seen: None,
            lowest_price_seen: None,
        };

        // Price dropped to 75000 after exit — good exit!
        let (since_exit_pct, _hypo_pnl, _diff, verdict) = compute_trade_followup(&trade, 75000.0);
        assert!(since_exit_pct < -10.0); // ~11.8% down since exit
        assert_eq!(verdict, "good_exit_significant");
    }

    #[test]
    fn test_followup_short_covered_early() {
        let trade = PaperTrade {
            id: 3,
            symbol: "AAPL".into(),
            side: "sell".into(),
            quantity: 100.0,
            entry_price: 200.0,
            exit_price: Some(190.0),
            reasoning: String::new(),
            confidence: 6,
            entry_time: "2026-01-01".into(),
            exit_time: Some("2026-01-10".into()),
            realized_pnl: Some(1000.0),
            stop_loss: None,
            take_profit: None,
            trailing_stop_pct: None,
            highest_price_seen: None,
            lowest_price_seen: None,
        };

        // Price dropped further to 170 — covered too early
        let (since_exit_pct, _hypo_pnl, _diff, verdict) = compute_trade_followup(&trade, 170.0);
        assert!(since_exit_pct < -10.0);
        assert_eq!(verdict, "covered_early_significant");
    }

    #[test]
    fn test_followup_neutral() {
        let trade = PaperTrade {
            id: 4,
            symbol: "ethereum".into(),
            side: "buy".into(),
            quantity: 10.0,
            entry_price: 2000.0,
            exit_price: Some(2100.0),
            reasoning: String::new(),
            confidence: 5,
            entry_time: "2026-01-01".into(),
            exit_time: Some("2026-01-10".into()),
            realized_pnl: Some(1000.0),
            stop_loss: None,
            take_profit: None,
            trailing_stop_pct: None,
            highest_price_seen: None,
            lowest_price_seen: None,
        };

        // Price barely moved — neutral
        let (_since_exit_pct, _hypo_pnl, _diff, verdict) = compute_trade_followup(&trade, 2110.0);
        assert_eq!(verdict, "neutral"); // < 2% change
    }

    #[test]
    fn test_equity_curve_empty() {
        let p = Portfolio::new();
        let curve = p.equity_curve();
        assert_eq!(curve.len(), 1);
        assert_eq!(curve[0].1, 100_000.0);
    }

    #[test]
    fn test_equity_curve_single_trade() {
        let mut p = Portfolio::new();
        let id = p.open_trade("AAPL", "buy", 10.0, 200.0, "Test", 5).unwrap();
        let curve = p.equity_curve();
        // Should have: Start + open event = 2 points
        assert_eq!(curve.len(), 2);
        assert_eq!(curve[0].1, 100_000.0); // Start
        // After buying 10*200=2000, cash is 98000, but position is worth 2000, total 100000
        assert!((curve[1].1 - 100_000.0).abs() < 0.01);

        // Close the trade with a profit
        p.close_trade(id, 220.0).unwrap();
        let curve = p.equity_curve();
        // Should have: Start + open + close = 3 points
        assert_eq!(curve.len(), 3);
        // After closing: cash = 98000 + 2200 = 100200
        assert!((curve[2].1 - 100_200.0).abs() < 0.01);
    }

    #[test]
    fn test_equity_curve_multiple_trades() {
        let mut p = Portfolio::new();
        // Trade 1: Win
        let id1 = p.open_trade("AAPL", "buy", 10.0, 100.0, "W", 5).unwrap();
        p.close_trade(id1, 110.0).unwrap(); // +100
        // Trade 2: Loss
        let id2 = p.open_trade("MSFT", "buy", 5.0, 200.0, "L", 5).unwrap();
        p.close_trade(id2, 190.0).unwrap(); // -50

        let curve = p.equity_curve();
        // Start + open1 + close1 + open2 + close2 = 5 points
        assert_eq!(curve.len(), 5);
        assert_eq!(curve[0].1, 100_000.0);
        // Final should be 100_000 + 100 - 50 = 100_050
        assert!((curve[4].1 - 100_050.0).abs() < 0.01);
    }

    #[test]
    fn test_equity_chart_empty() {
        let p = Portfolio::new();
        let chart = p.equity_chart();
        assert!(chart.contains("Equity Curve"));
        assert!(chart.contains("No trade events"));
    }

    #[test]
    fn test_equity_chart_with_trades() {
        let mut p = Portfolio::new();
        let id = p.open_trade("AAPL", "buy", 10.0, 100.0, "Test", 5).unwrap();
        p.close_trade(id, 110.0).unwrap();
        let chart = p.equity_chart();
        assert!(chart.contains("Equity Curve"));
        assert!(chart.contains("Start:"));
        assert!(chart.contains("Sparkline:"));
        assert!(chart.contains("Max Drawdown"));
    }
}
