//! Command handlers for yoyo slash commands.
//!
//! Extracted from main.rs to keep the codebase manageable.
//! Each handle_* function processes a specific slash command and its subcommands.

use crate::tools;
use yoagent::types::AgentTool;

// ANSI color helpers (shared with main.rs)
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const RED: &str = "\x1b[31m";

/// Execute a tool directly and print its output. Used by slash commands.
pub async fn execute_tool_direct(tool: &dyn yoagent::types::AgentTool, params: serde_json::Value) {
    let ctx = yoagent::types::ToolContext {
        tool_call_id: "direct".into(),
        tool_name: tool.name().into(),
        cancel: tokio_util::sync::CancellationToken::new(),
        on_update: None,
        on_progress: None,
    };
    match tool.execute(params, ctx).await {
        Ok(result) => {
            for c in &result.content {
                if let yoagent::types::Content::Text { text } = c {
                    println!("\n{text}\n");
                }
            }
        }
        Err(e) => println!("{RED}  Error: {e}{RESET}\n"),
    }
}

/// Handle /watchlist, /watch, /wl commands.
///
/// Subcommands:
///   /watchlist             — show current watchlist with prices
///   /watchlist add <sym>   — add a symbol
///   /watchlist rm <sym>    — remove a symbol
///   /wl                    — shorthand for /watchlist
pub async fn handle_watchlist_command(input: &str) {
    // Strip the command prefix to get the arguments
    let args = input
        .trim_start_matches("/watchlist")
        .trim_start_matches("/watch")
        .trim_start_matches("/wl")
        .trim();

    let parts: Vec<&str> = args.split_whitespace().collect();

    match parts.first().copied() {
        Some("add") | Some("+") => {
            if parts.len() < 2 {
                println!("{DIM}  Usage: /watchlist add bitcoin  or  /watchlist add AAPL{RESET}\n");
                return;
            }
            let mut wl = tools::watchlist::Watchlist::load();
            let symbol = parts[1];
            if wl.add(symbol) {
                if let Err(e) = wl.save() {
                    println!("{RED}  Error saving watchlist: {e}{RESET}\n");
                    return;
                }
                println!(
                    "{GREEN}  ✓ Added '{symbol}' to watchlist ({} total){RESET}\n",
                    wl.len()
                );
            } else {
                println!("{DIM}  '{symbol}' is already in your watchlist{RESET}\n");
            }
        }
        Some("rm") | Some("remove") | Some("-") => {
            if parts.len() < 2 {
                println!("{DIM}  Usage: /watchlist rm bitcoin  or  /watchlist rm AAPL{RESET}\n");
                return;
            }
            let mut wl = tools::watchlist::Watchlist::load();
            let symbol = parts[1];
            if wl.remove(symbol) {
                if let Err(e) = wl.save() {
                    println!("{RED}  Error saving watchlist: {e}{RESET}\n");
                    return;
                }
                println!(
                    "{GREEN}  ✓ Removed '{symbol}' from watchlist ({} remaining){RESET}\n",
                    wl.len()
                );
            } else {
                println!("{DIM}  '{symbol}' was not in your watchlist{RESET}\n");
            }
        }
        Some("clear") => {
            let mut wl = tools::watchlist::Watchlist::load();
            let count = wl.len();
            wl.symbols.clear();
            if let Err(e) = wl.save() {
                println!("{RED}  Error saving watchlist: {e}{RESET}\n");
                return;
            }
            println!("{GREEN}  ✓ Cleared watchlist ({count} symbols removed){RESET}\n");
        }
        None | Some("show") | Some("list") => {
            // Show watchlist with prices
            let wl = tools::watchlist::Watchlist::load();
            if wl.is_empty() {
                println!("\n{DIM}  📋 Your watchlist is empty.{RESET}");
                println!("{DIM}  Add symbols with: /watchlist add bitcoin{RESET}");
                println!("{DIM}  Or: /wl + AAPL{RESET}\n");
                return;
            }

            println!("\n{BOLD}{CYAN}  📋 Watchlist ({} assets){RESET}", wl.len());
            println!("{DIM}  ─────────────────────────────────────────{RESET}");
            println!("{DIM}  Fetching prices...{RESET}");

            // Fetch all prices concurrently
            let symbols: Vec<String> = wl.symbols.iter().cloned().collect();
            let futures: Vec<_> = symbols
                .iter()
                .map(|sym| {
                    let s = sym.clone();
                    async move {
                        let tool = tools::GetPriceTool::new();
                        let ctx = yoagent::types::ToolContext {
                            tool_call_id: "direct".into(),
                            tool_name: "get_price".into(),
                            cancel: tokio_util::sync::CancellationToken::new(),
                            on_update: None,
                            on_progress: None,
                        };
                        let result = tool.execute(serde_json::json!({"symbol": s}), ctx).await;
                        (s, result)
                    }
                })
                .collect();
            let results = futures::future::join_all(futures).await;

            for (sym, result) in &results {
                match result {
                    Ok(r) => {
                        for c in &r.content {
                            if let yoagent::types::Content::Text { text } = c {
                                // Print first two lines (emoji+name and price)
                                let lines: Vec<&str> = text.lines().collect();
                                if lines.len() >= 2 {
                                    println!("  {}", lines[0]);
                                    println!("    {}", lines[1]);
                                    if let Some(change_line) = lines.get(2) {
                                        println!("    {}", change_line);
                                    }
                                } else {
                                    for line in lines {
                                        println!("  {line}");
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => println!("  {RED}❌ {sym}: {e}{RESET}"),
                }
                println!();
            }
            println!("{DIM}  ─────────────────────────────────────────{RESET}");
            println!("{DIM}  Manage: /wl + <sym> | /wl - <sym> | /wl clear{RESET}\n");
        }
        Some(unknown) => {
            // Treat it as "add" if it looks like a symbol
            if unknown.len() <= 15
                && unknown
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '-' || c == '.' || c == '^')
            {
                let mut wl = tools::watchlist::Watchlist::load();
                if wl.add(unknown) {
                    if let Err(e) = wl.save() {
                        println!("{RED}  Error saving watchlist: {e}{RESET}\n");
                        return;
                    }
                    println!(
                        "{GREEN}  ✓ Added '{unknown}' to watchlist ({} total){RESET}\n",
                        wl.len()
                    );
                } else {
                    println!("{DIM}  '{unknown}' is already in your watchlist{RESET}\n");
                }
            } else {
                println!("{DIM}  Unknown watchlist command: {unknown}");
                println!("  Usage: /watchlist [add|rm|clear] [symbol]{RESET}\n");
            }
        }
    }
}

/// Handle /portfolio, /pf, /trade commands.
///
/// Subcommands:
///   /portfolio             — show portfolio summary (with live prices!)
///   /pf buy <sym> <qty> [price] [reason]  — open a buy position (auto-fetches price if omitted)
///   /pf sell <sym> <qty> [price] [reason] — open a short position (auto-fetches price if omitted)
///   /pf close <id> [price]                — close a position (auto-fetches price if omitted)
///   /pf reset                             — reset to starting balance
pub async fn handle_portfolio_command(input: &str) {
    let args = input
        .trim_start_matches("/portfolio")
        .trim_start_matches("/trade")
        .trim_start_matches("/pf")
        .trim();

    let parts: Vec<&str> = args.split_whitespace().collect();

    match parts.first().copied() {
        Some("buy") | Some("sell") => {
            let side = parts[0];
            if parts.len() < 3 {
                println!("{DIM}  Usage: /pf {side} <symbol> <quantity> [price] [reason]{RESET}");
                println!(
                    "{DIM}  Example: /pf buy bitcoin 0.5          (auto-fetches live price){RESET}"
                );
                println!("{DIM}  Example: /pf buy bitcoin 0.5 87000    (manual price){RESET}");
                println!("{DIM}  Example: /pf buy AAPL 10 BTC looks bullish  (auto-price + reason){RESET}\n");
                return;
            }
            let symbol = parts[1];
            let quantity: f64 = match parts[2].parse() {
                Ok(q) => q,
                Err(_) => {
                    println!("{RED}  Error: quantity must be a number{RESET}\n");
                    return;
                }
            };

            // Check if parts[3] is a number (manual price) or text (reason with auto-price)
            let (price, reason_start) = if parts.len() > 3 {
                if let Ok(p) = parts[3].parse::<f64>() {
                    (p, 4) // Manual price provided
                } else {
                    // parts[3] is not a number — it's the start of a reason, auto-fetch price
                    match fetch_live_price_for_trade(symbol).await {
                        Some(p) => (p, 3),
                        None => return,
                    }
                }
            } else {
                // No price or reason — auto-fetch
                match fetch_live_price_for_trade(symbol).await {
                    Some(p) => (p, parts.len()),
                    None => return,
                }
            };

            let reasoning = if parts.len() > reason_start {
                parts[reason_start..].join(" ")
            } else {
                String::new()
            };

            let mut portfolio = tools::portfolio::Portfolio::load();
            match portfolio.open_trade(symbol, side, quantity, price, &reasoning, 5) {
                Ok(id) => {
                    if let Err(e) = portfolio.save() {
                        println!("{RED}  Error saving portfolio: {e}{RESET}\n");
                        return;
                    }
                    // Log to TRADES.md
                    if let Some(trade) = portfolio.trades.iter().find(|t| t.id == id) {
                        let _ = tools::portfolio::log_trade_to_journal(trade, "open");
                    }
                    let notional = quantity * price;
                    println!(
                        "\n{GREEN}  ✓ Trade #{id} opened: {side} {symbol} x{quantity} @ ${price:.2} (${notional:.2}){RESET}"
                    );
                    println!("{DIM}  Cash remaining: ${:.2}{RESET}", portfolio.cash);

                    // Show risk assessment
                    let portfolio_value = portfolio.cash + notional; // approximate
                    let risk = tools::risk::assess_trade_risk(
                        portfolio_value,
                        notional,
                        price,
                        None, // No SL yet — will prompt
                        None, // Could fetch prices for indicator analysis
                    );
                    println!("{}", risk.format());
                    if risk.score >= 6 {
                        println!("{YELLOW}  💡 Consider setting a stop-loss: /pf sl {id} <price>{RESET}\n");
                    } else {
                        println!();
                    }
                }
                Err(e) => println!("{RED}  Error: {e}{RESET}\n"),
            }
        }
        Some("close") => {
            if parts.len() < 2 {
                println!("{DIM}  Usage: /pf close <trade_id> [exit_price]{RESET}");
                println!("{DIM}  Example: /pf close 1         (auto-fetches live price){RESET}");
                println!("{DIM}  Example: /pf close 1 92000   (manual price){RESET}\n");
                return;
            }
            let trade_id: u32 = match parts[1].trim_start_matches('#').parse() {
                Ok(id) => id,
                Err(_) => {
                    println!("{RED}  Error: trade_id must be a number{RESET}\n");
                    return;
                }
            };

            // Auto-fetch price if not provided
            let exit_price: f64 = if parts.len() >= 3 {
                match parts[2].parse() {
                    Ok(p) => p,
                    Err(_) => {
                        println!("{RED}  Error: exit_price must be a number{RESET}\n");
                        return;
                    }
                }
            } else {
                // Look up the symbol from the trade and auto-fetch
                let portfolio = tools::portfolio::Portfolio::load();
                let trade = match portfolio
                    .trades
                    .iter()
                    .find(|t| t.id == trade_id && t.is_open())
                {
                    Some(t) => t,
                    None => {
                        println!("{RED}  Error: No open trade found with ID #{trade_id}{RESET}\n");
                        return;
                    }
                };
                match fetch_live_price_for_trade(&trade.symbol).await {
                    Some(p) => p,
                    None => return,
                }
            };

            let mut portfolio = tools::portfolio::Portfolio::load();
            match portfolio.close_trade(trade_id, exit_price) {
                Ok(pnl) => {
                    if let Err(e) = portfolio.save() {
                        println!("{RED}  Error saving portfolio: {e}{RESET}\n");
                        return;
                    }
                    // Log to TRADES.md
                    if let Some(trade) = portfolio.trades.iter().find(|t| t.id == trade_id) {
                        let _ = tools::portfolio::log_trade_to_journal(trade, "close");
                    }
                    let pnl_emoji = if pnl >= 0.0 { "🟢" } else { "🔴" };
                    let pnl_sign = if pnl >= 0.0 { "+" } else { "" };
                    println!("\n{GREEN}  ✓ Trade #{trade_id} closed at ${exit_price:.2}{RESET}");
                    println!("  {pnl_emoji} P&L: {pnl_sign}${pnl:.2}");
                    println!("{DIM}  Cash: ${:.2}{RESET}\n", portfolio.cash);
                }
                Err(e) => println!("{RED}  Error: {e}{RESET}\n"),
            }
        }
        Some("sl") | Some("stoploss") | Some("stop-loss") => {
            if parts.len() < 3 {
                println!("{DIM}  Usage: /pf sl <trade_id> <price>{RESET}");
                println!("{DIM}  Example: /pf sl 1 85000{RESET}");
                println!("{DIM}  Use /pf sl <trade_id> off to remove{RESET}\n");
                return;
            }
            let trade_id: u32 = match parts[1].trim_start_matches('#').parse() {
                Ok(id) => id,
                Err(_) => {
                    println!("{RED}  Error: trade_id must be a number{RESET}\n");
                    return;
                }
            };
            let mut portfolio = tools::portfolio::Portfolio::load();
            if parts[2] == "off" || parts[2] == "none" || parts[2] == "remove" {
                match portfolio
                    .trades
                    .iter_mut()
                    .find(|t| t.id == trade_id && t.is_open())
                {
                    Some(t) => {
                        t.stop_loss = None;
                    }
                    None => {
                        println!("{RED}  Error: No open trade found with ID #{trade_id}{RESET}\n");
                        return;
                    }
                }
                if let Err(e) = portfolio.save() {
                    println!("{RED}  Error saving: {e}{RESET}\n");
                    return;
                }
                println!("{GREEN}  ✓ Stop-loss removed from trade #{trade_id}{RESET}\n");
            } else {
                let sl: f64 = match parts[2].parse() {
                    Ok(p) => p,
                    Err(_) => {
                        println!("{RED}  Error: price must be a number{RESET}\n");
                        return;
                    }
                };
                // Extract trade info and validate, then set SL
                let (entry_price, side, quantity) = {
                    let trade = match portfolio
                        .trades
                        .iter()
                        .find(|t| t.id == trade_id && t.is_open())
                    {
                        Some(t) => t,
                        None => {
                            println!(
                                "{RED}  Error: No open trade found with ID #{trade_id}{RESET}\n"
                            );
                            return;
                        }
                    };
                    (trade.entry_price, trade.side.clone(), trade.quantity)
                };
                if side == "buy" && sl >= entry_price {
                    println!("{RED}  Error: Stop-loss must be below entry price ${entry_price:.2} for a buy{RESET}\n");
                    return;
                }
                if side == "sell" && sl <= entry_price {
                    println!("{RED}  Error: Stop-loss must be above entry price ${entry_price:.2} for a short{RESET}\n");
                    return;
                }
                if let Some(trade) = portfolio.trades.iter_mut().find(|t| t.id == trade_id) {
                    trade.stop_loss = Some(sl);
                }
                if let Err(e) = portfolio.save() {
                    println!("{RED}  Error saving: {e}{RESET}\n");
                    return;
                }
                let risk = (entry_price - sl).abs() * quantity;
                println!("{GREEN}  ✓ Stop-loss set on trade #{trade_id}: ${sl:.2} (risk: ${risk:.2}){RESET}\n");
            }
        }
        Some("tp") | Some("takeprofit") | Some("take-profit") => {
            if parts.len() < 3 {
                println!("{DIM}  Usage: /pf tp <trade_id> <price>{RESET}");
                println!("{DIM}  Example: /pf tp 1 100000{RESET}");
                println!("{DIM}  Use /pf tp <trade_id> off to remove{RESET}\n");
                return;
            }
            let trade_id: u32 = match parts[1].trim_start_matches('#').parse() {
                Ok(id) => id,
                Err(_) => {
                    println!("{RED}  Error: trade_id must be a number{RESET}\n");
                    return;
                }
            };
            let mut portfolio = tools::portfolio::Portfolio::load();
            if parts[2] == "off" || parts[2] == "none" || parts[2] == "remove" {
                match portfolio
                    .trades
                    .iter_mut()
                    .find(|t| t.id == trade_id && t.is_open())
                {
                    Some(t) => {
                        t.take_profit = None;
                    }
                    None => {
                        println!("{RED}  Error: No open trade found with ID #{trade_id}{RESET}\n");
                        return;
                    }
                }
                if let Err(e) = portfolio.save() {
                    println!("{RED}  Error saving: {e}{RESET}\n");
                    return;
                }
                println!("{GREEN}  ✓ Take-profit removed from trade #{trade_id}{RESET}\n");
            } else {
                let tp: f64 = match parts[2].parse() {
                    Ok(p) => p,
                    Err(_) => {
                        println!("{RED}  Error: price must be a number{RESET}\n");
                        return;
                    }
                };
                // Extract trade info and validate, then set TP
                let (entry_price, side, quantity) = {
                    let trade = match portfolio
                        .trades
                        .iter()
                        .find(|t| t.id == trade_id && t.is_open())
                    {
                        Some(t) => t,
                        None => {
                            println!(
                                "{RED}  Error: No open trade found with ID #{trade_id}{RESET}\n"
                            );
                            return;
                        }
                    };
                    (trade.entry_price, trade.side.clone(), trade.quantity)
                };
                if side == "buy" && tp <= entry_price {
                    println!("{RED}  Error: Take-profit must be above entry price ${entry_price:.2} for a buy{RESET}\n");
                    return;
                }
                if side == "sell" && tp >= entry_price {
                    println!("{RED}  Error: Take-profit must be below entry price ${entry_price:.2} for a short{RESET}\n");
                    return;
                }
                if let Some(trade) = portfolio.trades.iter_mut().find(|t| t.id == trade_id) {
                    trade.take_profit = Some(tp);
                }
                if let Err(e) = portfolio.save() {
                    println!("{RED}  Error saving: {e}{RESET}\n");
                    return;
                }
                let reward = (tp - entry_price).abs() * quantity;
                println!("{GREEN}  ✓ Take-profit set on trade #{trade_id}: ${tp:.2} (target: +${reward:.2}){RESET}\n");
            }
        }
        Some("trail") | Some("trailing") | Some("tsl") => {
            if parts.len() < 3 {
                println!("{DIM}  Usage: /pf trail <trade_id> <percent>{RESET}");
                println!("{DIM}  Example: /pf trail 1 5     (5% trailing stop){RESET}");
                println!("{DIM}  Example: /pf trail 1 3.5   (3.5% trailing stop){RESET}");
                println!("{DIM}  Use /pf trail <trade_id> off to remove{RESET}");
                println!("{DIM}  The stop-loss ratchets up as price moves in your favor.{RESET}\n");
                return;
            }
            let trade_id: u32 = match parts[1].trim_start_matches('#').parse() {
                Ok(id) => id,
                Err(_) => {
                    println!("{RED}  Error: trade_id must be a number{RESET}\n");
                    return;
                }
            };
            let mut portfolio = tools::portfolio::Portfolio::load();
            if parts[2] == "off" || parts[2] == "none" || parts[2] == "remove" {
                match portfolio
                    .trades
                    .iter_mut()
                    .find(|t| t.id == trade_id && t.is_open())
                {
                    Some(t) => {
                        t.trailing_stop_pct = None;
                    }
                    None => {
                        println!("{RED}  Error: No open trade found with ID #{trade_id}{RESET}\n");
                        return;
                    }
                }
                if let Err(e) = portfolio.save() {
                    println!("{RED}  Error saving: {e}{RESET}\n");
                    return;
                }
                println!("{GREEN}  ✓ Trailing stop removed from trade #{trade_id}{RESET}\n");
            } else {
                let trail_pct: f64 = match parts[2].parse() {
                    Ok(p) if p > 0.0 && p < 100.0 => p,
                    _ => {
                        println!("{RED}  Error: percent must be a positive number (e.g., 5 for 5%){RESET}\n");
                        return;
                    }
                };
                match portfolio
                    .trades
                    .iter_mut()
                    .find(|t| t.id == trade_id && t.is_open())
                {
                    Some(t) => {
                        t.trailing_stop_pct = Some(trail_pct);
                        // Initialize highest/lowest price seen if not already set
                        if t.highest_price_seen.is_none() {
                            t.highest_price_seen = Some(t.entry_price);
                        }
                        if t.lowest_price_seen.is_none() {
                            t.lowest_price_seen = Some(t.entry_price);
                        }
                        // Set initial trailing SL based on entry price
                        let initial_sl = if t.side == "buy" {
                            t.entry_price * (1.0 - trail_pct / 100.0)
                        } else {
                            t.entry_price * (1.0 + trail_pct / 100.0)
                        };
                        // Only set if no SL or trailing SL is better
                        let current_sl = t.stop_loss;
                        if t.side == "buy" {
                            if current_sl.is_none() || current_sl.unwrap() < initial_sl {
                                t.stop_loss = Some(initial_sl);
                            }
                        } else if current_sl.is_none() || current_sl.unwrap() > initial_sl {
                            t.stop_loss = Some(initial_sl);
                        }
                    }
                    None => {
                        println!("{RED}  Error: No open trade found with ID #{trade_id}{RESET}\n");
                        return;
                    }
                }
                if let Err(e) = portfolio.save() {
                    println!("{RED}  Error saving: {e}{RESET}\n");
                    return;
                }
                let trade = portfolio.trades.iter().find(|t| t.id == trade_id).unwrap();
                println!("{GREEN}  ✓ Trailing stop set on trade #{trade_id}: {trail_pct}%{RESET}");
                println!(
                    "{DIM}    Current SL: ${:.2}{RESET}",
                    trade.stop_loss.unwrap_or(0.0)
                );
                println!("{DIM}    The stop-loss will ratchet up as the price moves in your favor.{RESET}\n");
            }
        }
        Some("history") | Some("log") | Some("trades") => {
            let portfolio = tools::portfolio::Portfolio::load();
            let limit = if parts.len() >= 2 {
                parts[1].parse::<usize>().unwrap_or(20)
            } else {
                20
            };
            println!("\n{}", portfolio.history_report(limit));
        }
        Some("stats") | Some("performance") | Some("perf") => {
            let portfolio = tools::portfolio::Portfolio::load();
            println!("\n{}", portfolio.performance_report());
        }
        Some("analyze") | Some("patterns") | Some("mistakes") => {
            let portfolio = tools::portfolio::Portfolio::load();
            let report = tools::trade_analysis::analyze_trades(&portfolio);
            println!("\n{}", report.format());
        }
        Some("reset") => {
            let portfolio = tools::portfolio::Portfolio::new();
            if let Err(e) = portfolio.save() {
                println!("{RED}  Error saving portfolio: {e}{RESET}\n");
                return;
            }
            println!(
                "{GREEN}  ✓ Portfolio reset to ${:.2}{RESET}\n",
                portfolio.starting_balance
            );
        }
        None | Some("show") | Some("summary") => {
            let portfolio = tools::portfolio::Portfolio::load();
            let open = portfolio.open_positions();
            if open.is_empty() {
                println!("\n{}", portfolio.summary());
            } else {
                // Fetch live prices for open positions
                println!("{DIM}  Fetching live prices for open positions...{RESET}");
                let symbols: Vec<String> = open.iter().map(|t| t.symbol.clone()).collect();
                let unique_symbols: std::collections::HashSet<String> =
                    symbols.into_iter().collect();

                let futures: Vec<_> = unique_symbols
                    .into_iter()
                    .map(|sym| {
                        let s = sym.clone();
                        async move {
                            let result = tools::fetch_live_price(&s).await;
                            (s, result)
                        }
                    })
                    .collect();
                let results = futures::future::join_all(futures).await;

                let price_map: std::collections::HashMap<String, f64> = results
                    .into_iter()
                    .filter_map(|(sym, r)| r.ok().map(|(price, _)| (sym, price)))
                    .collect();

                // Check for stop-loss / take-profit / trailing-stop triggers
                let mut portfolio_mut = portfolio.clone();
                let triggered = portfolio_mut.check_stop_loss_take_profit(&price_map);
                if !triggered.is_empty() {
                    for (trade_id, trigger_price, trigger_type) in &triggered {
                        match portfolio_mut.close_trade(*trade_id, *trigger_price) {
                            Ok(pnl) => {
                                let pnl_emoji = if pnl >= 0.0 { "🟢" } else { "🔴" };
                                let pnl_sign = if pnl >= 0.0 { "+" } else { "" };
                                println!(
                                    "{YELLOW}  ⚡ {trigger_type} triggered for trade #{trade_id}!{RESET}"
                                );
                                println!(
                                    "  {pnl_emoji} Closed at ${trigger_price:.2} — P&L: {pnl_sign}${pnl:.2}"
                                );
                                // Log to TRADES.md
                                if let Some(trade) =
                                    portfolio_mut.trades.iter().find(|t| t.id == *trade_id)
                                {
                                    let _ = tools::portfolio::log_trade_to_journal(trade, "close");
                                }
                            }
                            Err(e) => {
                                println!("{RED}  Error auto-closing trade #{trade_id}: {e}{RESET}");
                            }
                        }
                    }
                    if let Err(e) = portfolio_mut.save() {
                        println!("{RED}  Error saving portfolio: {e}{RESET}");
                    }
                    println!();
                    // Use the updated portfolio for the summary
                    println!("{}", portfolio_mut.summary_with_prices(&price_map));
                } else {
                    // Save updated trailing stop state even if nothing triggered
                    if let Err(e) = portfolio_mut.save() {
                        println!("{RED}  Error saving portfolio: {e}{RESET}");
                    }
                    println!("\n{}", portfolio_mut.summary_with_prices(&price_map));
                }
            }
        }
        Some(unknown) => {
            println!("{DIM}  Unknown portfolio command: {unknown}");
            println!("  Usage: /portfolio [buy|sell|close|reset|show]{RESET}\n");
        }
    }
}

/// Helper to fetch a live price and print status messages.
/// Returns Some(price) on success, None on failure (with error already printed).
pub async fn fetch_live_price_for_trade(symbol: &str) -> Option<f64> {
    println!("{DIM}  Fetching live price for {symbol}...{RESET}");
    match tools::fetch_live_price(symbol).await {
        Ok((price, name)) => {
            println!("{GREEN}  📈 Live price: ${price:.2} ({name}){RESET}");
            Some(price)
        }
        Err(e) => {
            println!("{RED}  Error fetching price: {e}{RESET}");
            println!("{DIM}  Tip: specify price manually: /pf buy {symbol} <qty> <price>{RESET}\n");
            None
        }
    }
}

/// Handle /alert commands.
///
/// Subcommands:
///   /alert                          — show all alerts + check active ones
///   /alert <sym> above <price> [note] — alert when price goes above target
///   /alert <sym> below <price> [note] — alert when price goes below target
///   /alert rm <id>                  — remove an alert
///   /alert clear                    — clear all triggered alerts
pub async fn handle_alert_command(input: &str) {
    let args = input.trim_start_matches("/alert").trim();
    let parts: Vec<&str> = args.split_whitespace().collect();

    match parts.first().copied() {
        Some("rm") | Some("remove") | Some("delete") => {
            if parts.len() < 2 {
                println!("{DIM}  Usage: /alert rm <alert_id>{RESET}\n");
                return;
            }
            let alert_id: u32 = match parts[1].trim_start_matches('#').parse() {
                Ok(id) => id,
                Err(_) => {
                    println!("{RED}  Error: alert_id must be a number{RESET}\n");
                    return;
                }
            };
            let mut am = tools::alerts::AlertManager::load();
            if am.remove_alert(alert_id) {
                if let Err(e) = am.save() {
                    println!("{RED}  Error saving: {e}{RESET}\n");
                    return;
                }
                println!("{GREEN}  ✓ Alert #{alert_id} removed{RESET}\n");
            } else {
                println!("{DIM}  No alert found with ID #{alert_id}{RESET}\n");
            }
        }
        Some("clear") => {
            let mut am = tools::alerts::AlertManager::load();
            let count = am.triggered_alerts().len();
            am.clear_triggered();
            if let Err(e) = am.save() {
                println!("{RED}  Error saving: {e}{RESET}\n");
                return;
            }
            println!("{GREEN}  ✓ Cleared {count} triggered alerts{RESET}\n");
        }
        None | Some("show") | Some("list") => {
            // Show alerts and check active ones against live prices
            let mut am = tools::alerts::AlertManager::load();
            let active_symbols = am.active_symbols();

            if !active_symbols.is_empty() {
                println!("{DIM}  Checking prices for active alerts...{RESET}");
                let futures: Vec<_> = active_symbols
                    .into_iter()
                    .map(|sym| {
                        let s = sym.clone();
                        async move {
                            let result = tools::fetch_live_price(&s).await;
                            (s, result)
                        }
                    })
                    .collect();
                let results = futures::future::join_all(futures).await;

                let price_map: std::collections::HashMap<String, f64> = results
                    .into_iter()
                    .filter_map(|(sym, r)| r.ok().map(|(price, _)| (sym, price)))
                    .collect();

                let triggered = am.check_alerts(&price_map);
                if !triggered.is_empty() {
                    for (id, symbol, condition, target, current) in &triggered {
                        let emoji = if *condition == "above" {
                            "📈"
                        } else {
                            "📉"
                        };
                        println!(
                            "\n{YELLOW}  🔔 ALERT #{id}: {symbol} is {condition} ${target:.2}! Current: ${current:.2} {emoji}{RESET}"
                        );
                    }
                    if let Err(e) = am.save() {
                        println!("{RED}  Error saving: {e}{RESET}");
                    }
                    println!();
                }
            }

            println!("\n{}", am.format_alerts());
        }
        Some(symbol) => {
            // Try to parse: /alert <symbol> <above|below> <price> [note]
            if parts.len() < 3 {
                println!("{DIM}  Usage: /alert <symbol> <above|below> <price> [note]{RESET}");
                println!("{DIM}  Example: /alert bitcoin below 80000 Buy the dip{RESET}");
                println!("{DIM}  Example: /alert AAPL above 200{RESET}\n");
                return;
            }

            let condition = parts[1];
            if condition != "above" && condition != "below" {
                println!("{RED}  Error: condition must be 'above' or 'below'{RESET}\n");
                return;
            }

            let target_price: f64 = match parts[2].parse() {
                Ok(p) => p,
                Err(_) => {
                    println!("{RED}  Error: price must be a number{RESET}\n");
                    return;
                }
            };

            let note = if parts.len() > 3 {
                parts[3..].join(" ")
            } else {
                String::new()
            };

            let mut am = tools::alerts::AlertManager::load();
            match am.add_alert(symbol, condition, target_price, &note) {
                Ok(id) => {
                    if let Err(e) = am.save() {
                        println!("{RED}  Error saving: {e}{RESET}\n");
                        return;
                    }
                    let arrow = if condition == "above" { "↑" } else { "↓" };
                    println!(
                        "{GREEN}  ✓ Alert #{id}: {symbol} {arrow} {condition} ${target_price:.2}{RESET}"
                    );
                    if !note.is_empty() {
                        println!("{DIM}    Note: {note}{RESET}");
                    }
                    println!("{DIM}    Alerts are checked when you use /alert, /portfolio, or /watchlist{RESET}\n");
                }
                Err(e) => println!("{RED}  Error: {e}{RESET}\n"),
            }
        }
    }
}

/// Handle /correlate commands.
///
/// Usage:
///   /correlate <symbol1> <symbol2> [range]
///   /corr bitcoin ethereum 90d
pub async fn handle_correlate_command(input: &str) {
    let args = input
        .trim_start_matches("/correlate")
        .trim_start_matches("/corr")
        .trim();
    let parts: Vec<&str> = args.split_whitespace().collect();

    if parts.len() < 2 {
        println!("{DIM}  Usage: /correlate <symbol1> <symbol2> [range]{RESET}");
        println!("{DIM}  Example: /correlate bitcoin ethereum 90d{RESET}");
        println!("{DIM}  Example: /corr AAPL MSFT 1y{RESET}");
        println!("{DIM}  Ranges: 7d, 30d, 90d, 1y (default: 30d){RESET}\n");
        return;
    }

    let sym_a = parts[0];
    let sym_b = parts[1];
    let range = parts.get(2).copied().unwrap_or("30d");

    println!("{DIM}  Fetching price history for {sym_a} and {sym_b} ({range})...{RESET}");

    // Fetch both price histories concurrently
    let (result_a, result_b) = tokio::join!(
        fetch_price_series(sym_a, range),
        fetch_price_series(sym_b, range),
    );

    let prices_a = match result_a {
        Ok(p) => p,
        Err(e) => {
            println!("{RED}  Error fetching {sym_a}: {e}{RESET}\n");
            return;
        }
    };

    let prices_b = match result_b {
        Ok(p) => p,
        Err(e) => {
            println!("{RED}  Error fetching {sym_b}: {e}{RESET}\n");
            return;
        }
    };

    // Align series to the same length (take the shorter one)
    let min_len = prices_a.len().min(prices_b.len());
    if min_len < 5 {
        println!("{RED}  Error: Not enough data points for correlation (need at least 5, got {min_len}){RESET}\n");
        return;
    }

    // Use the most recent data points
    let a_slice = &prices_a[prices_a.len() - min_len..];
    let b_slice = &prices_b[prices_b.len() - min_len..];

    // Compute correlation on returns (% changes) — more meaningful than raw prices
    let returns_a = tools::indicators::returns(a_slice);
    let returns_b = tools::indicators::returns(b_slice);

    let corr_price = tools::indicators::correlation(a_slice, b_slice);
    let corr_returns = tools::indicators::correlation(&returns_a, &returns_b);

    println!();
    println!("{BOLD}{CYAN}  🔗 Correlation: {sym_a} vs {sym_b} ({range}){RESET}");
    println!("{DIM}  ─────────────────────────────────────────{RESET}");
    println!("  Data points: {min_len}");

    if let Some(r) = corr_price {
        println!(
            "  Price correlation:  {:.4} {}",
            r,
            tools::indicators::correlation_signal(r)
        );
    }
    if let Some(r) = corr_returns {
        println!(
            "  Return correlation: {:.4} {}",
            r,
            tools::indicators::correlation_signal(r)
        );
    }

    // Performance comparison
    let change_a = if a_slice[0] > 0.0 {
        ((a_slice[a_slice.len() - 1] - a_slice[0]) / a_slice[0]) * 100.0
    } else {
        0.0
    };
    let change_b = if b_slice[0] > 0.0 {
        ((b_slice[b_slice.len() - 1] - b_slice[0]) / b_slice[0]) * 100.0
    } else {
        0.0
    };
    println!(
        "  {sym_a} change: {}{:.2}%",
        if change_a >= 0.0 { "+" } else { "" },
        change_a
    );
    println!(
        "  {sym_b} change: {}{:.2}%",
        if change_b >= 0.0 { "+" } else { "" },
        change_b
    );

    println!("{DIM}  ─────────────────────────────────────────{RESET}");
    println!("{DIM}  Return correlation is more meaningful for trading decisions.{RESET}");
    println!("{DIM}  ⚠️  Correlation changes over time. Past correlation ≠ future.{RESET}\n");
}

/// Fetch a price series for correlation analysis.
/// Returns a Vec of close prices.
pub async fn fetch_price_series(symbol: &str, range: &str) -> Result<Vec<f64>, String> {
    use tools::format::is_likely_stock_ticker;
    use tools::http::create_client;

    let client = create_client();

    if is_likely_stock_ticker(symbol) {
        fetch_yahoo_price_series(&client, symbol, range).await
    } else {
        match fetch_coingecko_price_series(&client, symbol, range).await {
            Ok(prices) => Ok(prices),
            Err(_) => {
                let yahoo_sym = format!("{}-USD", symbol.to_uppercase());
                fetch_yahoo_price_series(&client, &yahoo_sym, range).await
            }
        }
    }
}

pub async fn fetch_coingecko_price_series(
    client: &reqwest::Client,
    coin_id: &str,
    range: &str,
) -> Result<Vec<f64>, String> {
    use tools::http::fetch_json_with_retry;

    let days = match range {
        "1d" => "1",
        "7d" => "7",
        "30d" => "30",
        "90d" => "90",
        "1y" => "365",
        _ => "30",
    };
    let url = format!(
        "https://api.coingecko.com/api/v3/coins/{}/market_chart?vs_currency=usd&days={}",
        coin_id.to_lowercase(),
        days,
    );

    let data = fetch_json_with_retry(client, &url).await?;
    let prices = data["prices"]
        .as_array()
        .ok_or_else(|| format!("No price data for '{}'", coin_id))?;

    let values: Vec<f64> = prices
        .iter()
        .filter_map(|p| p.as_array()?.get(1)?.as_f64())
        .collect();

    if values.is_empty() {
        return Err(format!("Empty price data for '{}'", coin_id));
    }
    Ok(values)
}

pub async fn fetch_yahoo_price_series(
    client: &reqwest::Client,
    symbol: &str,
    range: &str,
) -> Result<Vec<f64>, String> {
    use tools::http::fetch_json_with_retry;

    let (yahoo_range, interval) = match range {
        "1d" => ("1d", "5m"),
        "7d" => ("5d", "1h"),
        "30d" => ("1mo", "1d"),
        "90d" => ("3mo", "1d"),
        "1y" => ("1y", "1wk"),
        _ => ("1mo", "1d"),
    };
    let url = format!(
        "https://query1.finance.yahoo.com/v8/finance/chart/{}?range={}&interval={}",
        symbol, yahoo_range, interval
    );

    let data = fetch_json_with_retry(client, &url).await?;
    let result = data["chart"]["result"]
        .as_array()
        .and_then(|arr| arr.first())
        .ok_or_else(|| format!("No data for '{}'", symbol))?;

    let closes: Vec<f64> = result["indicators"]["quote"]
        .as_array()
        .and_then(|arr| arr.first())
        .and_then(|q| q["close"].as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_f64()).collect())
        .unwrap_or_default();

    if closes.is_empty() {
        return Err(format!("No close data for '{}'", symbol));
    }
    Ok(closes)
}

/// Handle /mtf (multi-timeframe) command.
///
/// Fetches 7d, 30d, and 90d data, computes signal summary for each,
/// and shows alignment across timeframes.
pub async fn handle_mtf_command(input: &str) {
    let args = input.trim_start_matches("/mtf").trim();
    let parts: Vec<&str> = args.split_whitespace().collect();

    if parts.is_empty() {
        println!("{DIM}  Usage: /mtf <symbol>{RESET}");
        println!("{DIM}  Example: /mtf bitcoin{RESET}");
        println!("{DIM}  Example: /mtf AAPL{RESET}");
        println!("{DIM}  Analyzes 7d, 30d, and 90d timeframes together.{RESET}\n");
        return;
    }

    let symbol = parts[0];
    println!("{DIM}  Fetching multi-timeframe data for {symbol}...{RESET}");

    // Fetch all three timeframes concurrently
    let (res_7d, res_30d, res_90d) = tokio::join!(
        fetch_price_series(symbol, "7d"),
        fetch_price_series(symbol, "30d"),
        fetch_price_series(symbol, "90d"),
    );

    println!();
    println!("{BOLD}{CYAN}  📊 Multi-Timeframe Analysis: {symbol}{RESET}");
    println!("{DIM}  ─────────────────────────────────────────{RESET}");

    let mut timeframe_results: Vec<(&str, Option<tools::SignalCounts>)> = Vec::new();

    for (label, result) in [("7d", res_7d), ("30d", res_30d), ("90d", res_90d)] {
        match result {
            Ok(prices) => {
                if prices.is_empty() {
                    println!("  {label:>4}: ❌ No data");
                    timeframe_results.push((label, None));
                    continue;
                }
                let current_price = *prices.last().unwrap();
                let change = if prices[0] > 0.0 {
                    ((current_price - prices[0]) / prices[0]) * 100.0
                } else {
                    0.0
                };

                let signal = tools::compute_signal_counts(&prices, current_price, None, None, None);

                if let Some(ref s) = signal {
                    println!(
                        "  {label:>4}: {} {} ({} bull, {} bear, {} neutral) | {}{:.2}%",
                        s.emoji,
                        s.verdict,
                        s.bullish,
                        s.bearish,
                        s.neutral,
                        if change >= 0.0 { "+" } else { "" },
                        change,
                    );
                } else {
                    println!(
                        "  {label:>4}: ⚪ Insufficient data for signals | {}{:.2}%",
                        if change >= 0.0 { "+" } else { "" },
                        change
                    );
                }
                timeframe_results.push((label, signal));
            }
            Err(e) => {
                println!("  {label:>4}: ❌ {e}");
                timeframe_results.push((label, None));
            }
        }
    }

    // Compute alignment score
    let valid_signals: Vec<&tools::SignalCounts> = timeframe_results
        .iter()
        .filter_map(|(_, s)| s.as_ref())
        .collect();

    if valid_signals.len() >= 2 {
        println!("{DIM}  ─────────────────────────────────────────{RESET}");

        let all_bullish = valid_signals.iter().all(|s| s.bullish > s.bearish);
        let all_bearish = valid_signals.iter().all(|s| s.bearish > s.bullish);
        let total_bullish: u32 = valid_signals.iter().map(|s| s.bullish).sum();
        let total_bearish: u32 = valid_signals.iter().map(|s| s.bearish).sum();

        if all_bullish {
            println!("  🟢 ALL TIMEFRAMES BULLISH — Strong trend alignment");
            println!(
                "  📊 Combined: {} bullish vs {} bearish across all timeframes",
                total_bullish, total_bearish
            );
        } else if all_bearish {
            println!("  🔴 ALL TIMEFRAMES BEARISH — Strong trend alignment");
            println!(
                "  📊 Combined: {} bearish vs {} bullish across all timeframes",
                total_bearish, total_bullish
            );
        } else {
            // Mixed — look for divergence
            let short_bullish = valid_signals
                .first()
                .map(|s| s.bullish > s.bearish)
                .unwrap_or(false);
            let long_bearish = valid_signals
                .last()
                .map(|s| s.bearish > s.bullish)
                .unwrap_or(false);
            let short_bearish = valid_signals
                .first()
                .map(|s| s.bearish > s.bullish)
                .unwrap_or(false);
            let long_bullish = valid_signals
                .last()
                .map(|s| s.bullish > s.bearish)
                .unwrap_or(false);

            if short_bullish && long_bearish {
                println!("  🟡 DIVERGENCE: Short-term bullish, long-term bearish");
                println!("  💡 Could be a dead-cat bounce or early reversal. Caution advised.");
            } else if short_bearish && long_bullish {
                println!("  🟡 DIVERGENCE: Short-term bearish, long-term bullish");
                println!("  💡 Possible pullback in an uptrend. Watch for buying opportunity.");
            } else {
                println!("  ⚪ MIXED SIGNALS across timeframes");
                println!(
                    "  📊 Combined: {} bullish, {} bearish",
                    total_bullish, total_bearish
                );
            }
        }
    }

    println!("{DIM}  ─────────────────────────────────────────{RESET}");
    println!("{DIM}  Multi-timeframe convergence = stronger signal.{RESET}");
    println!("{DIM}  ⚠️  Not financial advice. Always do your own research.{RESET}\n");
}

/// Handle /risk command — assess risk for a proposed trade.
///
/// Usage:
///   /risk <symbol> <quantity> [price] [stop_loss]
///   /risk bitcoin 0.5 87000 82000
///   /risk AAPL 100
pub async fn handle_risk_command(input: &str) {
    let args = input.trim_start_matches("/risk").trim();
    let parts: Vec<&str> = args.split_whitespace().collect();

    if parts.len() < 2 {
        println!("{DIM}  Usage: /risk <symbol> <quantity> [price] [stop_loss]{RESET}");
        println!("{DIM}  Example: /risk bitcoin 0.5 87000 82000{RESET}");
        println!("{DIM}  Example: /risk AAPL 100  (auto-fetches price){RESET}\n");
        return;
    }

    let symbol = parts[0];
    let quantity: f64 = match parts[1].parse() {
        Ok(q) if q > 0.0 => q,
        _ => {
            println!("{RED}  Error: quantity must be a positive number{RESET}\n");
            return;
        }
    };

    // Parse or auto-fetch price
    let price = if parts.len() >= 3 {
        match parts[2].parse::<f64>() {
            Ok(p) if p > 0.0 => p,
            _ => {
                println!("{RED}  Error: price must be a positive number{RESET}\n");
                return;
            }
        }
    } else {
        match fetch_live_price_for_trade(symbol).await {
            Some(p) => p,
            None => return,
        }
    };

    let stop_loss: Option<f64> = if parts.len() >= 4 {
        match parts[3].parse::<f64>() {
            Ok(sl) if sl > 0.0 => Some(sl),
            _ => {
                println!("{RED}  Error: stop_loss must be a positive number{RESET}\n");
                return;
            }
        }
    } else {
        None
    };

    let notional = quantity * price;
    let portfolio = tools::portfolio::Portfolio::load();
    let portfolio_value = portfolio.cash
        + portfolio
            .open_positions()
            .iter()
            .map(|t| t.notional_value())
            .sum::<f64>();

    // Try to fetch price history for indicator-based risk assessment
    println!("{DIM}  Fetching price data for risk analysis...{RESET}");
    let prices_opt = match fetch_price_series(symbol, "30d").await {
        Ok(p) if p.len() >= 30 => Some(p),
        _ => None,
    };

    let risk = tools::risk::assess_trade_risk(
        portfolio_value,
        notional,
        price,
        stop_loss,
        prices_opt.as_deref(),
    );

    println!();
    println!(
        "{BOLD}{CYAN}  ⚖️  Risk Assessment: {} {} x{} @ ${:.2}{RESET}",
        symbol,
        if quantity > 0.0 { "BUY" } else { "SELL" },
        quantity,
        price,
    );
    println!(
        "{DIM}  Trade Value: ${:.2} | Portfolio: ${:.2}{RESET}",
        notional, portfolio_value
    );
    println!("{DIM}  ─────────────────────────────────────────{RESET}");
    print!("{}", risk.format());
    println!("{DIM}  ─────────────────────────────────────────{RESET}");
    println!("{DIM}  ⚠️  Not financial advice. Always do your own research.{RESET}\n");
}

/// Handle /dashboard, /dash, /status — unified status overview.
///
/// Shows portfolio balance, open positions, watchlist prices, active alerts,
/// and recent trade stats in one view.
pub async fn handle_dashboard_command() {
    println!();
    println!("{BOLD}{CYAN}  ══════════════════════════════════════════{RESET}");
    println!("{BOLD}{CYAN}  📊 yoyo Dashboard{RESET}");
    println!("{BOLD}{CYAN}  ══════════════════════════════════════════{RESET}");

    // 1. Portfolio Summary
    let mut portfolio = tools::portfolio::Portfolio::load();
    let open = portfolio.open_positions().to_vec();
    let closed = portfolio.closed_positions().to_vec();
    let realized_pnl = portfolio.total_realized_pnl();

    println!();
    println!("{BOLD}  💼 Portfolio{RESET}");
    println!("{DIM}  ─────────────────────────────────────────{RESET}");
    println!(
        "  Cash: {}  |  Open: {}  |  Closed: {}",
        tools::format::format_currency_unsigned(portfolio.cash),
        open.len(),
        closed.len(),
    );
    if !closed.is_empty() {
        let wr = portfolio.win_rate().unwrap_or(0.0);
        println!(
            "  Realized P&L: {}  |  Win Rate: {:.1}%",
            tools::format::format_currency(realized_pnl),
            wr,
        );
    }

    // 2. Open Positions with live prices
    if !open.is_empty() {
        println!();
        println!("{BOLD}  📈 Open Positions{RESET}");
        println!("{DIM}  ─────────────────────────────────────────{RESET}");

        // Gather unique symbols for price fetching
        let symbols: std::collections::HashSet<String> =
            open.iter().map(|t| t.symbol.clone()).collect();
        let futures: Vec<_> = symbols
            .into_iter()
            .map(|sym| {
                let s = sym.clone();
                async move {
                    let result = tools::fetch_live_price(&s).await;
                    (s, result)
                }
            })
            .collect();
        let results = futures::future::join_all(futures).await;
        let price_map: std::collections::HashMap<String, f64> = results
            .into_iter()
            .filter_map(|(sym, r)| r.ok().map(|(price, _)| (sym, price)))
            .collect();

        let mut total_unrealized = 0.0;
        for trade in &open {
            let pnl_info = if let Some(&current_price) = price_map.get(&trade.symbol) {
                let upnl = trade.unrealized_pnl(current_price);
                total_unrealized += upnl;
                let pnl_pct = trade.pnl_pct(current_price);
                let emoji = if upnl >= 0.0 { "🟢" } else { "🔴" };
                format!(
                    "{} {} ({}{:.2}%)",
                    emoji,
                    tools::format::format_currency(upnl),
                    if pnl_pct >= 0.0 { "+" } else { "" },
                    pnl_pct,
                )
            } else {
                "⚪ (no price)".to_string()
            };
            println!(
                "  #{} {} {} x{:.4} @ {} — {}",
                trade.id,
                trade.side.to_uppercase(),
                trade.symbol,
                trade.quantity,
                tools::format::format_currency_unsigned(trade.entry_price),
                pnl_info,
            );
        }
        println!(
            "  {DIM}Unrealized total: {}{RESET}",
            tools::format::format_currency(total_unrealized),
        );

        // Check for SL/TP triggers
        let triggered = portfolio.check_stop_loss_take_profit(&price_map);
        if !triggered.is_empty() {
            for (trade_id, _price, trigger_type) in &triggered {
                println!(
                    "  {YELLOW}⚡ Trade #{} {} triggered!{RESET}",
                    trade_id, trigger_type
                );
            }
        }
    }

    // 3. Watchlist
    let wl = tools::watchlist::Watchlist::load();
    if !wl.is_empty() {
        println!();
        println!("{BOLD}  📋 Watchlist ({} assets){RESET}", wl.len());
        println!("{DIM}  ─────────────────────────────────────────{RESET}");

        let symbols: Vec<String> = wl.symbols.iter().cloned().collect();
        let futures: Vec<_> = symbols
            .iter()
            .map(|sym| {
                let s = sym.clone();
                async move {
                    let result = tools::fetch_live_price(&s).await;
                    (s, result)
                }
            })
            .collect();
        let results = futures::future::join_all(futures).await;

        for (sym, result) in &results {
            match result {
                Ok((price, name)) => {
                    println!("  {} — {}", name, tools::format::format_price(*price),);
                }
                Err(_) => {
                    println!("  {sym} — ❌ error");
                }
            }
        }
    }

    // 4. Active Alerts
    let am = tools::alerts::AlertManager::load();
    let active = am.active_alerts();
    if !active.is_empty() {
        println!();
        println!("{BOLD}  🔔 Active Alerts ({} pending){RESET}", active.len());
        println!("{DIM}  ─────────────────────────────────────────{RESET}");
        for alert in &active {
            let arrow = if alert.condition == "above" {
                "↑"
            } else {
                "↓"
            };
            println!(
                "  #{} {} {} {}",
                alert.id,
                alert.symbol,
                arrow,
                tools::format::format_currency_unsigned(alert.target_price),
            );
        }
    }

    println!();
    println!("{BOLD}{CYAN}  ══════════════════════════════════════════{RESET}");
    println!("{DIM}  Commands: /portfolio | /watchlist | /alert | /help{RESET}\n");
}

/// Handle /backtest commands.
///
/// Usage:
///   /backtest <symbol> [strategy] [range]
///   /bt bitcoin sma 90d
///   /bt AAPL rsi 1y
///   /bt bitcoin sma_10_30 90d
///   /bt bitcoin compare 90d    — compare ALL strategies
pub async fn handle_backtest_command(input: &str) {
    let args = input
        .trim_start_matches("/backtest")
        .trim_start_matches("/bt")
        .trim();
    let parts: Vec<&str> = args.split_whitespace().collect();

    if parts.is_empty() {
        println!("\n{BOLD}{CYAN}  🧪 Backtest — Test strategies against historical data{RESET}");
        println!("{DIM}  ─────────────────────────────────────────{RESET}");
        println!("{DIM}  Usage: /backtest <symbol> [strategy] [range]{RESET}");
        println!("{DIM}  Example: /backtest bitcoin sma 90d{RESET}");
        println!("{DIM}  Example: /bt AAPL rsi 1y{RESET}");
        println!("{DIM}  Example: /bt bitcoin sma_10_30 90d{RESET}");
        println!("{DIM}  Example: /bt bitcoin compare 90d    — compare ALL strategies{RESET}");
        println!();
        println!("{DIM}  Available strategies:{RESET}");
        for (name, desc) in tools::backtest::available_strategies() {
            println!("    {BOLD}{name}{RESET}  — {desc}");
        }
        println!("    {BOLD}compare{RESET}  — Run ALL strategies and show ranked comparison");
        println!();
        println!("{DIM}  Ranges: 7d, 30d, 90d, 1y (default: 90d){RESET}");
        println!("{DIM}  Default strategy: sma (SMA Crossover 7/25){RESET}\n");
        return;
    }

    let symbol = parts[0];
    let strategy_str = parts.get(1).copied().unwrap_or("sma");
    let range = parts.get(2).copied().unwrap_or("90d");

    // Handle "compare" mode — run ALL strategies
    if strategy_str == "compare" || strategy_str == "cmp" || strategy_str == "all" {
        println!("{DIM}  Fetching {symbol} history ({range}) for strategy comparison...{RESET}");

        match fetch_price_series(symbol, range).await {
            Ok(prices) => {
                if prices.len() < 30 {
                    println!("{RED}  Not enough data for backtesting (need 30+ data points, got {}){RESET}\n", prices.len());
                    return;
                }
                println!(
                    "{DIM}  Running 6 strategies on {} data points...{RESET}",
                    prices.len()
                );
                let result = tools::backtest::run_comparison(&prices, symbol, range);
                println!("\n{}", result.format());
            }
            Err(e) => {
                println!("{RED}  Error fetching data: {e}{RESET}\n");
            }
        }
        return;
    }

    let strategy = match tools::backtest::parse_strategy(strategy_str) {
        Some(s) => s,
        None => {
            println!("{RED}  Unknown strategy: '{strategy_str}'{RESET}");
            println!("{DIM}  Available: sma, sma_10_30, rsi, rsi_14_25_75, bb, compare{RESET}\n");
            return;
        }
    };

    println!("{DIM}  Fetching {symbol} history ({range}) for backtesting...{RESET}");

    match fetch_price_series(symbol, range).await {
        Ok(prices) => {
            if prices.len() < 30 {
                println!("{RED}  Not enough data for backtesting (need 30+ data points, got {}){RESET}\n", prices.len());
                return;
            }
            println!(
                "{DIM}  Running {} on {} data points...{RESET}",
                strategy.name(),
                prices.len()
            );
            let result = tools::backtest::run_backtest(&prices, &strategy, symbol, range);
            println!("\n{}", result.format());
        }
        Err(e) => {
            println!("{RED}  Error fetching data: {e}{RESET}\n");
        }
    }
}

/// Handle /size command — position sizing calculator.
///
/// Usage:
///   /size <symbol> <entry_price> <stop_loss> [risk_pct] [take_profit]
///   /size bitcoin 87000 85000 2
///   /size AAPL 200 190 1.5 220
pub async fn handle_size_command(input: &str) {
    let args = input.trim_start_matches("/size").trim();
    let parts: Vec<&str> = args.split_whitespace().collect();

    if parts.len() < 3 {
        println!("{DIM}  Usage: /size <symbol> <entry_price> <stop_loss> [risk_pct] [take_profit]{RESET}");
        println!("{DIM}  Example: /size bitcoin 87000 85000 2{RESET}");
        println!("{DIM}  Example: /size AAPL 200 190 1.5 220{RESET}");
        println!("{DIM}  risk_pct default: 2% of portfolio{RESET}\n");
        return;
    }

    let symbol = parts[0];

    // Check if parts[1] is "auto" or a number
    let entry_price: f64 = if parts[1] == "auto" || parts[1] == "live" {
        match fetch_live_price_for_trade(symbol).await {
            Some(p) => p,
            None => return,
        }
    } else {
        match parts[1].parse() {
            Ok(p) if p > 0.0 => p,
            _ => {
                println!(
                    "{RED}  Error: entry_price must be a positive number (or 'auto'){RESET}\n"
                );
                return;
            }
        }
    };

    let stop_loss: f64 = match parts[2].parse() {
        Ok(p) if p > 0.0 => p,
        _ => {
            println!("{RED}  Error: stop_loss must be a positive number{RESET}\n");
            return;
        }
    };

    if entry_price == stop_loss {
        println!("{RED}  Error: entry price and stop-loss cannot be the same{RESET}\n");
        return;
    }

    let risk_pct: f64 = if parts.len() >= 4 {
        match parts[3].parse() {
            Ok(p) if p > 0.0 && p <= 100.0 => p,
            _ => {
                println!("{RED}  Error: risk_pct must be between 0 and 100{RESET}\n");
                return;
            }
        }
    } else {
        2.0 // Default 2% risk
    };

    let take_profit: Option<f64> = if parts.len() >= 5 {
        match parts[4].parse::<f64>() {
            Ok(p) if p > 0.0 => Some(p),
            _ => {
                println!("{RED}  Error: take_profit must be a positive number{RESET}\n");
                return;
            }
        }
    } else {
        None
    };

    let portfolio = tools::portfolio::Portfolio::load();
    let portfolio_value = portfolio.cash
        + portfolio
            .open_positions()
            .iter()
            .map(|t| t.notional_value())
            .sum::<f64>();

    match tools::risk::calculate_position_size(
        portfolio_value,
        entry_price,
        stop_loss,
        risk_pct,
        take_profit,
        symbol,
    ) {
        Ok(sizing) => {
            println!();
            println!("{}", sizing.format());

            // Try to fetch price data and show stop-loss suggestions for context
            match fetch_price_series(symbol, "30d").await {
                Ok(prices) if prices.len() >= 15 => {
                    let side = if stop_loss < entry_price {
                        "buy"
                    } else {
                        "sell"
                    };
                    println!(
                        "{}",
                        tools::risk::suggest_stop_loss_levels(entry_price, &prices, side)
                    );
                }
                _ => {}
            }
        }
        Err(e) => {
            println!("{RED}  Error: {e}{RESET}\n");
        }
    }
}

/// Handle /suggest or /idea command — auto-generate a trade idea with entry/SL/TP.
///
/// Fetches 30d price data, runs technical analysis, and provides a structured
/// recommendation with position sizing based on current portfolio.
///
/// Usage:
///   /suggest <symbol>
///   /idea bitcoin
pub async fn handle_suggest_command(input: &str) {
    let args = input
        .trim_start_matches("/suggest")
        .trim_start_matches("/idea")
        .trim();

    if args.is_empty() {
        println!("{DIM}  Usage: /suggest <symbol>{RESET}");
        println!("{DIM}  Example: /suggest bitcoin{RESET}");
        println!("{DIM}  Example: /idea AAPL{RESET}");
        println!("{DIM}  Generates a trade idea with entry, stop-loss, and take-profit.{RESET}\n");
        return;
    }

    let symbol = args.split_whitespace().next().unwrap_or(args);
    println!("{DIM}  Analyzing {symbol} for a trade suggestion...{RESET}");

    // Fetch price data for multiple timeframes
    let (res_7d, res_30d, res_90d) = tokio::join!(
        fetch_price_series(symbol, "7d"),
        fetch_price_series(symbol, "30d"),
        fetch_price_series(symbol, "90d"),
    );

    let prices_30d = match res_30d {
        Ok(p) if p.len() >= 20 => p,
        Ok(p) => {
            println!(
                "{RED}  Not enough data for analysis (need 20+ points, got {}){RESET}\n",
                p.len()
            );
            return;
        }
        Err(e) => {
            println!("{RED}  Error fetching {symbol}: {e}{RESET}\n");
            return;
        }
    };

    let current_price = *prices_30d.last().unwrap();

    // Get signal counts for each timeframe
    let signal_7d = res_7d.ok().and_then(|p| {
        if p.len() >= 10 {
            let current = *p.last().unwrap();
            tools::compute_signal_counts(&p, current, None, None, None)
        } else {
            None
        }
    });

    let signal_30d = tools::compute_signal_counts(&prices_30d, current_price, None, None, None);

    let signal_90d = res_90d.ok().and_then(|p| {
        if p.len() >= 20 {
            let current = *p.last().unwrap();
            tools::compute_signal_counts(&p, current, None, None, None)
        } else {
            None
        }
    });

    // Determine overall bias
    let mut bullish_count = 0u32;
    let mut bearish_count = 0u32;
    let mut total_timeframes = 0u32;

    for signal in [&signal_7d, &signal_30d, &signal_90d]
        .iter()
        .copied()
        .flatten()
    {
        total_timeframes += 1;
        if signal.bullish > signal.bearish {
            bullish_count += 1;
        } else if signal.bearish > signal.bullish {
            bearish_count += 1;
        }
    }

    // Compute RSI and SMA for additional context
    let rsi = tools::indicators::rsi(&prices_30d, 14);
    let sma_7 = tools::indicators::sma(&prices_30d, 7);
    let sma_20 = tools::indicators::sma(&prices_30d, 20);

    // Determine action
    let (action, confidence, reasoning) = if total_timeframes == 0 {
        ("HOLD", 3u8, "Insufficient data for analysis.".to_string())
    } else if bullish_count >= 2 && bearish_count == 0 {
        let conf = if bullish_count == 3 { 8 } else { 7 };
        let reason = format!(
            "Bullish across {}/{} timeframes. {}",
            bullish_count,
            total_timeframes,
            if rsi.map_or(false, |r| r < 70.0) {
                "RSI not overbought — room to run."
            } else {
                "⚠️ RSI elevated — watch for pullback."
            }
        );
        ("BUY", conf, reason)
    } else if bearish_count >= 2 && bullish_count == 0 {
        let conf = if bearish_count == 3 { 8 } else { 7 };
        let reason = format!(
            "Bearish across {}/{} timeframes. {}",
            bearish_count,
            total_timeframes,
            if rsi.map_or(false, |r| r > 30.0) {
                "RSI not oversold — downtrend may continue."
            } else {
                "⚠️ RSI deeply oversold — bounce possible."
            }
        );
        ("SELL/AVOID", conf, reason)
    } else if bullish_count > bearish_count {
        (
            "CAUTIOUS BUY",
            5,
            format!(
                "Mixed signals — {} bullish, {} bearish timeframes. Wait for clearer setup.",
                bullish_count, bearish_count
            ),
        )
    } else if bearish_count > bullish_count {
        (
            "CAUTIOUS SELL",
            5,
            format!(
                "Mixed signals — {} bearish, {} bullish timeframes. Risk of reversal.",
                bearish_count, bullish_count
            ),
        )
    } else {
        (
            "HOLD",
            4,
            "Neutral across timeframes. No clear edge.".to_string(),
        )
    };

    // Calculate suggested levels
    let volatility_proxy = {
        let changes: Vec<f64> = prices_30d.windows(2).map(|w| (w[1] - w[0]).abs()).collect();
        changes.iter().sum::<f64>() / changes.len() as f64
    };

    let (suggested_sl, suggested_tp) = if action.contains("BUY") {
        let sl = current_price - volatility_proxy * 2.0;
        let tp = current_price + volatility_proxy * 4.0;
        (sl, tp)
    } else if action.contains("SELL") {
        let sl = current_price + volatility_proxy * 2.0;
        let tp = current_price - volatility_proxy * 4.0;
        (sl, tp)
    } else {
        let sl = current_price * 0.95;
        let tp = current_price * 1.10;
        (sl, tp)
    };

    let risk_per_unit = (current_price - suggested_sl).abs();
    let reward_per_unit = (suggested_tp - current_price).abs();
    let rr_ratio = if risk_per_unit > 0.0 {
        reward_per_unit / risk_per_unit
    } else {
        0.0
    };

    // Portfolio-based sizing
    let portfolio = tools::portfolio::Portfolio::load();
    let portfolio_value = portfolio.cash
        + portfolio
            .open_positions()
            .iter()
            .map(|t| t.notional_value())
            .sum::<f64>();
    let risk_budget = portfolio_value * 0.02; // 2% risk
    let suggested_qty = if risk_per_unit > 0.0 {
        risk_budget / risk_per_unit
    } else {
        0.0
    };
    let notional = suggested_qty * current_price;

    // Output
    println!();
    let action_emoji = match action {
        a if a.contains("BUY") => "🟢",
        a if a.contains("SELL") => "🔴",
        _ => "⚪",
    };
    println!("{BOLD}{CYAN}  💡 Trade Suggestion: {symbol}{RESET}");
    println!("{DIM}  ═════════════════════════════════════════{RESET}");
    println!("  {action_emoji} Action: {BOLD}{action}{RESET}  (confidence: {confidence}/10)");
    println!("  📝 {reasoning}");
    println!("{DIM}  ─────────────────────────────────────────{RESET}");

    // Show timeframe breakdown
    for (label, signal) in [
        ("7d", &signal_7d),
        ("30d", &signal_30d),
        ("90d", &signal_90d),
    ] {
        if let Some(s) = signal {
            println!(
                "  {label:>4}: {} {} ({} bull, {} bear, {} neutral)",
                s.emoji, s.verdict, s.bullish, s.bearish, s.neutral
            );
        }
    }

    if let Some(r) = rsi {
        println!("  RSI: {:.1} {}", r, tools::indicators::rsi_signal(r));
    }
    if let (Some(s7), Some(s20)) = (sma_7, sma_20) {
        let trend = tools::indicators::sma_signal(current_price, s7, s20);
        println!("  Trend: {}", trend);
    }

    println!("{DIM}  ─────────────────────────────────────────{RESET}");
    println!("  📊 Suggested Levels:");
    println!(
        "    Entry:       {}",
        tools::format::format_price(current_price),
    );
    println!(
        "    Stop-Loss:   {} ({:.2}% risk)",
        tools::format::format_price(suggested_sl),
        (risk_per_unit / current_price) * 100.0,
    );
    println!(
        "    Take-Profit: {} ({:.2}% reward)",
        tools::format::format_price(suggested_tp),
        (reward_per_unit / current_price) * 100.0,
    );
    println!("    Risk/Reward: 1:{:.2}", rr_ratio);

    println!("{DIM}  ─────────────────────────────────────────{RESET}");
    println!("  📐 Position Sizing (2% risk budget):");
    println!("    Quantity:   {:.6} units", suggested_qty,);
    println!(
        "    Notional:   {} ({:.1}% of portfolio)",
        tools::format::format_currency_unsigned(notional),
        if portfolio_value > 0.0 {
            (notional / portfolio_value) * 100.0
        } else {
            0.0
        },
    );
    println!(
        "    Max Loss:   {}",
        tools::format::format_currency_unsigned(risk_budget),
    );

    println!("{DIM}  ═════════════════════════════════════════{RESET}");
    if action.contains("BUY") && confidence >= 7 {
        println!("  💡 To execute: /pf buy {symbol} {:.6}", suggested_qty);
        println!("     Then set SL: /pf sl <id> {:.2}", suggested_sl);
        println!("     And TP:      /pf tp <id> {:.2}", suggested_tp);
    } else if action.contains("SELL") && confidence >= 7 {
        println!("  💡 To execute: /pf sell {symbol} {:.6}", suggested_qty);
    } else {
        println!("  💡 Confidence is low — consider waiting for a clearer setup.");
    }
    println!("{DIM}  ⚠️  Not financial advice. Do your own research.{RESET}\n");
}

/// Handle /scan or /screener command — scan multiple assets for signals.
///
/// Usage:
///   /scan                     — scan watchlist
///   /scan bitcoin ethereum solana AAPL MSFT
///   /screener bitcoin ethereum
pub async fn handle_scan_command(input: &str) {
    let args = input
        .trim_start_matches("/screener")
        .trim_start_matches("/scan")
        .trim();

    let symbols: Vec<String> = if args.is_empty() {
        // Default to watchlist
        let wl = tools::watchlist::Watchlist::load();
        if wl.is_empty() {
            println!("\n{DIM}  📋 Your watchlist is empty. Add symbols or specify them directly:{RESET}");
            println!("{DIM}  /scan bitcoin ethereum solana AAPL MSFT{RESET}");
            println!("{DIM}  Or: /wl + bitcoin  to add to watchlist first{RESET}\n");
            return;
        }
        wl.symbols.iter().cloned().collect()
    } else {
        args.split_whitespace().map(|s| s.to_string()).collect()
    };

    if symbols.is_empty() {
        println!("{DIM}  Usage: /scan <symbol1> <symbol2> ... or /scan (uses watchlist){RESET}\n");
        return;
    }

    println!("{DIM}  Scanning {} assets for signals (30d data)...{RESET}", symbols.len());

    // Fetch price data for all symbols concurrently
    let futures: Vec<_> = symbols
        .iter()
        .map(|sym| {
            let s = sym.clone();
            async move {
                let prices = fetch_price_series(&s, "30d").await;
                let live = tools::fetch_live_price(&s).await;
                (s, prices, live)
            }
        })
        .collect();
    let results = futures::future::join_all(futures).await;

    // Build scan results
    struct ScanResult {
        symbol: String,
        name: String,
        price: f64,
        change_pct: f64,
        signal: Option<tools::SignalCounts>,
        rsi: Option<f64>,
    }

    let mut scan_results: Vec<ScanResult> = Vec::new();

    for (sym, prices_result, live_result) in results {
        let (price, name) = match live_result {
            Ok((p, n)) => (p, n),
            Err(_) => continue,
        };

        let (signal, change_pct, rsi) = match prices_result {
            Ok(prices) if prices.len() >= 15 => {
                let current = *prices.last().unwrap_or(&price);
                let change = if prices[0] > 0.0 {
                    ((current - prices[0]) / prices[0]) * 100.0
                } else {
                    0.0
                };
                let sig = tools::compute_signal_counts(&prices, current, None, None, None);
                let rsi_val = tools::indicators::rsi(&prices, 14);
                (sig, change, rsi_val)
            }
            _ => (None, 0.0, None),
        };

        scan_results.push(ScanResult {
            symbol: sym,
            name,
            price,
            change_pct,
            signal,
            rsi,
        });
    }

    if scan_results.is_empty() {
        println!("{RED}  No results — couldn't fetch data for any of the symbols.{RESET}\n");
        return;
    }

    // Sort by signal strength: strong bullish first, then slight bullish, neutral, slight bearish, strong bearish
    scan_results.sort_by(|a, b| {
        let score_a = match &a.signal {
            Some(s) => s.bullish as i32 - s.bearish as i32,
            None => -100,
        };
        let score_b = match &b.signal {
            Some(s) => s.bullish as i32 - s.bearish as i32,
            None => -100,
        };
        score_b.cmp(&score_a)
    });

    // Print results
    println!();
    println!("{BOLD}{CYAN}  🔍 Signal Scanner ({} assets){RESET}", scan_results.len());
    println!("{DIM}  ═════════════════════════════════════════════════════════════{RESET}");
    println!(
        "{BOLD}  {:<16} {:>10} {:>8} {:>5}  {:>6}  {}{RESET}",
        "Asset", "Price", "30d Chg", "RSI", "Signal", "Indicators"
    );
    println!("{DIM}  ─────────────────────────────────────────────────────────────{RESET}");

    for result in &scan_results {
        let price_str = tools::format::format_price(result.price);
        let change_str = format!(
            "{}{}%",
            if result.change_pct >= 0.0 { "+" } else { "" },
            format!("{:.1}", result.change_pct),
        );
        let rsi_str = match result.rsi {
            Some(r) => format!("{:.0}", r),
            None => "—".to_string(),
        };
        let (verdict_str, indicator_str) = match &result.signal {
            Some(s) => {
                let dots: String = s
                    .signals
                    .iter()
                    .map(|(_, dot)| dot.as_str())
                    .collect::<Vec<&str>>()
                    .join("");
                (
                    format!("{} {}", s.emoji, &s.verdict[..s.verdict.len().min(12)]),
                    dots,
                )
            }
            None => ("⚪ N/A".to_string(), String::new()),
        };

        // Truncate name for display, include symbol if different
        let display_name = if result.symbol == result.name {
            if result.name.len() > 15 {
                format!("{}…", &result.name[..14])
            } else {
                result.name.clone()
            }
        } else if result.name.len() > 15 {
            // Show symbol instead of long name
            result.symbol.clone()
        } else {
            result.name.clone()
        };

        println!(
            "  {:<16} {:>10} {:>8} {:>5}  {}  {}",
            display_name, price_str, change_str, rsi_str, verdict_str, indicator_str,
        );
    }

    // Summary
    let bullish_count = scan_results
        .iter()
        .filter(|r| r.signal.as_ref().map_or(false, |s| s.bullish > s.bearish))
        .count();
    let bearish_count = scan_results
        .iter()
        .filter(|r| r.signal.as_ref().map_or(false, |s| s.bearish > s.bullish))
        .count();
    let neutral_count = scan_results.len() - bullish_count - bearish_count;

    println!("{DIM}  ─────────────────────────────────────────────────────────────{RESET}");
    println!(
        "  Summary: {} 🟢 bullish | {} 🔴 bearish | {} ⚪ neutral/mixed",
        bullish_count, bearish_count, neutral_count,
    );

    // Highlight strongest signals
    if let Some(most_bullish) = scan_results.iter().find(|r| {
        r.signal
            .as_ref()
            .map_or(false, |s| s.bullish > s.bearish + 1)
    }) {
        println!(
            "  🟢 Strongest bullish: {} ({})",
            most_bullish.name,
            most_bullish
                .signal
                .as_ref()
                .map(|s| s.verdict.clone())
                .unwrap_or_default(),
        );
    }
    if let Some(most_bearish) = scan_results.iter().rev().find(|r| {
        r.signal
            .as_ref()
            .map_or(false, |s| s.bearish > s.bullish + 1)
    }) {
        println!(
            "  🔴 Strongest bearish: {} ({})",
            most_bearish.name,
            most_bearish
                .signal
                .as_ref()
                .map(|s| s.verdict.clone())
                .unwrap_or_default(),
        );
    }

    println!("{DIM}  ═════════════════════════════════════════════════════════════{RESET}");
    println!("{DIM}  Use /ta <symbol> for detailed analysis on any asset above.{RESET}");
    println!("{DIM}  ⚠️  Not financial advice. Always do your own research.{RESET}\n");
}
