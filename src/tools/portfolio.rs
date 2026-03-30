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

    /// Check if any open trades should be closed based on stop-loss or take-profit.
    /// Returns a list of (trade_id, trigger_price, trigger_type) for trades that should close.
    pub fn check_stop_loss_take_profit(
        &self,
        price_map: &std::collections::HashMap<String, f64>,
    ) -> Vec<(u32, f64, &'static str)> {
        let mut triggered = Vec::new();

        for trade in &self.trades {
            if !trade.is_open() {
                continue;
            }
            if let Some(&current_price) = price_map.get(&trade.symbol) {
                // Check stop-loss
                if let Some(sl) = trade.stop_loss {
                    let triggered_sl = if trade.side == "buy" {
                        current_price <= sl
                    } else {
                        current_price >= sl
                    };
                    if triggered_sl {
                        triggered.push((trade.id, current_price, "stop-loss"));
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
        let mut output = String::new();
        output.push_str("💼 Paper Trading Portfolio\n");
        output.push_str("─────────────────────────────────────────\n");
        output.push_str(&format!(
            "  Starting Balance: ${:.2}\n",
            self.starting_balance
        ));
        output.push_str(&format!("  Cash Available:   ${:.2}\n", self.cash));

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
            "  Realized P&L:     {}{:.2}\n",
            if realized >= 0.0 { "+$" } else { "-$" },
            realized.abs()
        ));
        if !open.is_empty() && !price_map.is_empty() {
            output.push_str(&format!(
                "  Unrealized P&L:   {}{:.2}\n",
                if total_unrealized >= 0.0 { "+$" } else { "-$" },
                total_unrealized.abs()
            ));
            let total_pnl = realized + total_unrealized;
            let total_value = self.cash + total_position_value;
            let total_return =
                ((total_value - self.starting_balance) / self.starting_balance) * 100.0;
            output.push_str(&format!(
                "  Total P&L:        {}{:.2} ({}{:.2}%)\n",
                if total_pnl >= 0.0 { "+$" } else { "-$" },
                total_pnl.abs(),
                if total_return >= 0.0 { "+" } else { "" },
                total_return,
            ));
            output.push_str(&format!("  Portfolio Value:  ${:.2}\n", total_value));
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
                        " → ${:.2} {} {}{:.2} ({}{:.2}%)",
                        current_price,
                        emoji,
                        if upnl >= 0.0 { "+$" } else { "-$" },
                        upnl.abs(),
                        if pnl_pct >= 0.0 { "+" } else { "" },
                        pnl_pct,
                    )
                } else {
                    String::new()
                };

                output.push_str(&format!(
                    "    #{} {} {} x{:.4} @ ${:.2}{}\n",
                    trade.id,
                    trade.side.to_uppercase(),
                    trade.symbol,
                    trade.quantity,
                    trade.entry_price,
                    pnl_info,
                ));
                // Show SL/TP if set
                let mut levels = Vec::new();
                if let Some(sl) = trade.stop_loss {
                    levels.push(format!("SL: ${:.2}", sl));
                }
                if let Some(tp) = trade.take_profit {
                    levels.push(format!("TP: ${:.2}", tp));
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
                    "    #{} {} {} x{:.4} @ ${:.2} → ${:.2} {} {}{:.2}\n",
                    trade.id,
                    trade.side.to_uppercase(),
                    trade.symbol,
                    trade.quantity,
                    trade.entry_price,
                    trade.exit_price.unwrap_or(0.0),
                    pnl_emoji,
                    if pnl >= 0.0 { "+$" } else { "-$" },
                    pnl.abs(),
                ));
            }
        }

        output.push_str("─────────────────────────────────────────\n");
        output.push_str("  ⚠️  Paper trading only — no real money at risk.\n");

        output
    }

    /// Get portfolio summary as formatted text.
    pub fn summary(&self) -> String {
        let mut output = String::new();
        output.push_str("💼 Paper Trading Portfolio\n");
        output.push_str("─────────────────────────────────────────\n");
        output.push_str(&format!(
            "  Starting Balance: ${:.2}\n",
            self.starting_balance
        ));
        output.push_str(&format!("  Cash Available:   ${:.2}\n", self.cash));

        let open = self.open_positions();
        let closed = self.closed_positions();
        let realized = self.total_realized_pnl();

        output.push_str(&format!("  Open Positions:   {}\n", open.len()));
        output.push_str(&format!("  Closed Trades:    {}\n", closed.len()));
        output.push_str(&format!(
            "  Realized P&L:     {}{:.2}\n",
            if realized >= 0.0 { "+$" } else { "-$" },
            realized.abs()
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
                    "    #{} {} {} x{:.4} @ ${:.2} ({})\n",
                    trade.id,
                    trade.side.to_uppercase(),
                    trade.symbol,
                    trade.quantity,
                    trade.entry_price,
                    trade.entry_time,
                ));
                // Show SL/TP if set
                let mut levels = Vec::new();
                if let Some(sl) = trade.stop_loss {
                    levels.push(format!("SL: ${:.2}", sl));
                }
                if let Some(tp) = trade.take_profit {
                    levels.push(format!("TP: ${:.2}", tp));
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
                    "    #{} {} {} x{:.4} @ ${:.2} → ${:.2} {} {}{:.2}\n",
                    trade.id,
                    trade.side.to_uppercase(),
                    trade.symbol,
                    trade.quantity,
                    trade.entry_price,
                    trade.exit_price.unwrap_or(0.0),
                    pnl_emoji,
                    if pnl >= 0.0 { "+$" } else { "-$" },
                    pnl.abs(),
                ));
            }
        }

        output.push_str("─────────────────────────────────────────\n");
        output.push_str("  ⚠️  Paper trading only — no real money at risk.\n");

        output
    }
}

/// Get current timestamp as ISO 8601 string.
fn current_timestamp() -> String {
    // Use a simple format without chrono dependency
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Convert to approximate date-time (good enough for paper trading)
    let secs_per_day = 86400u64;
    let days_since_epoch = now / secs_per_day;
    let remaining_secs = now % secs_per_day;
    let hours = remaining_secs / 3600;
    let mins = (remaining_secs % 3600) / 60;

    // Approximate year/month/day calculation
    let mut year = 1970u64;
    let mut remaining_days = days_since_epoch;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let month_days = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1u64;
    for &days in &month_days {
        if remaining_days < days {
            break;
        }
        remaining_days -= days;
        month += 1;
    }
    let day = remaining_days + 1;

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}Z",
        year, month, day, hours, mins
    )
}

fn is_leap_year(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
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
        assert!(summary.contains("100000"));
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
        assert!(summary.contains("+$5,100") || summary.contains("+$5100"));
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
        assert!(summary.contains("-$5,000") || summary.contains("-$5000"));
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
}
