//! yoyo — a coding agent that evolves itself.
//!
//! Started as ~200 lines. Grows one commit at a time.
//! Read IDENTITY.md, JOURNAL.md, and ROADMAP.md for the full story.
//!
//! Usage:
//!   ANTHROPIC_API_KEY=sk-... cargo run
//!   ANTHROPIC_API_KEY=sk-... cargo run -- --model claude-opus-4-6
//!   ANTHROPIC_API_KEY=sk-... cargo run -- --skills ./skills
//!
//! Commands:
//!   /quit, /exit    Exit the agent
//!   /clear          Clear conversation history
//!   /model <name>   Switch model mid-session

mod proxy_provider;
mod tools;

use std::io::{self, BufRead, Read, Write};
use yoagent::agent::Agent;
use yoagent::provider::{AnthropicProvider, ModelConfig, OpenAiCompat, OpenAiCompatProvider};
use yoagent::skills::SkillSet;
use yoagent::tools::default_tools;
use yoagent::*;

use proxy_provider::ProxyAnthropicProvider;

// ANSI color helpers
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const CYAN: &str = "\x1b[36m";
const RED: &str = "\x1b[31m";

const SYSTEM_PROMPT: &str = r#"You are yoyo, an AI trading companion for crypto and US stocks.

You have real-time market data tools:
- **get_price**: Fetch current price, 24h change, market cap for any crypto or stock
- **get_price_history**: Fetch OHLCV historical data with sparkline charts (1d, 7d, 30d, 90d, 1y)
- **search_symbol**: Find the right symbol/ID for any asset by name
- **get_market_overview**: Quick snapshot of top crypto + major US indices
- **get_news**: Fetch latest news headlines for any asset or market topic

You also have coding tools (bash, read_file, write_file, edit_file, search, list_files).

**How to help:**
- When someone asks about a price, USE the get_price tool — don't guess
- When someone asks about price history or trends, USE get_price_history
- When someone wants a market overview, USE get_market_overview
- When someone mentions an asset you're not sure about, USE search_symbol to find it
- When someone asks about news or what's happening, USE get_news to fetch headlines
- Be conversational but data-driven. Show the numbers, then explain what they mean
- Always remind users you're not a financial advisor and trading carries risk

**Your personality:** Direct, curious, honest about uncertainty. You track your own accuracy and learn from mistakes. You remember users and their interests (see MEMORY.md)."#;

fn print_banner() {
    println!("\n{BOLD}{CYAN}  yoyo{RESET} {DIM}— your AI trading companion (v0.20.0){RESET}");
    println!("{DIM}  Type /help for commands, or just chat naturally{RESET}\n");
}

fn print_usage(usage: &Usage) {
    if usage.input > 0 || usage.output > 0 {
        println!(
            "\n{DIM}  tokens: {} in / {} out{RESET}",
            usage.input, usage.output
        );
    }
}

#[tokio::main]
async fn main() {
    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .or_else(|_| std::env::var("API_KEY"))
        .expect("Set ANTHROPIC_API_KEY or API_KEY");

    let args: Vec<String> = std::env::args().collect();

    let model = args
        .iter()
        .position(|a| a == "--model")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "claude-opus-4-6".into());

    let provider_name = args
        .iter()
        .position(|a| a == "--provider")
        .and_then(|i| args.get(i + 1))
        .cloned()
        .unwrap_or_else(|| "anthropic".into());

    let base_url = args
        .iter()
        .position(|a| a == "--base-url")
        .and_then(|i| args.get(i + 1))
        .cloned();

    let skill_dirs: Vec<String> = args
        .iter()
        .enumerate()
        .filter(|(_, a)| a.as_str() == "--skills")
        .filter_map(|(i, _)| args.get(i + 1).cloned())
        .collect();

    let skills = if skill_dirs.is_empty() {
        SkillSet::empty()
    } else {
        SkillSet::load(&skill_dirs).expect("Failed to load skills")
    };

    let mut agent = build_agent(
        &provider_name,
        &model,
        &api_key,
        base_url.as_deref(),
        &skills,
    );

    print_banner();
    println!("{DIM}  provider: {provider_name}{RESET}");
    println!("{DIM}  model: {model}{RESET}");
    if let Some(url) = &base_url {
        println!("{DIM}  base_url: {url}{RESET}");
    }
    if !skills.is_empty() {
        println!("{DIM}  skills: {} loaded{RESET}", skills.len());
    }
    println!(
        "{DIM}  cwd:   {}{RESET}\n",
        std::env::current_dir().unwrap().display()
    );

    let stdin = io::stdin();
    let is_pipe = !atty::is(atty::Stream::Stdin);

    // When stdin is a pipe (e.g. from evolve.sh), read ALL input as one prompt.
    // When interactive (TTY), read line by line as a REPL.
    if is_pipe {
        let mut full_input = String::new();
        stdin.lock().read_to_string(&mut full_input).ok();
        let input = full_input.trim();
        if !input.is_empty() {
            println!("{DIM}  (piped input: {} chars){RESET}", input.len());
            let mut rx = agent.prompt(input).await;
            let mut last_usage = Usage::default();
            let mut in_text = false;

            while let Some(event) = rx.recv().await {
                match event {
                    AgentEvent::ToolExecutionStart {
                        tool_name, args, ..
                    } => {
                        if in_text {
                            println!();
                            in_text = false;
                        }
                        let summary = format_tool_summary(&tool_name, &args);
                        print!("{YELLOW}  ▶ {summary}{RESET}");
                        io::stdout().flush().ok();
                    }
                    AgentEvent::ToolExecutionEnd { is_error, .. } => {
                        if is_error {
                            println!(" {RED}✗{RESET}");
                        } else {
                            println!(" {GREEN}✓{RESET}");
                        }
                    }
                    AgentEvent::MessageUpdate {
                        delta: StreamDelta::Text { delta },
                        ..
                    } => {
                        if !in_text {
                            println!();
                            in_text = true;
                        }
                        print!("{}", delta);
                        io::stdout().flush().ok();
                    }
                    AgentEvent::AgentEnd { messages } => {
                        for msg in messages.iter().rev() {
                            if let AgentMessage::Llm(Message::Assistant { usage, .. }) = msg {
                                last_usage = usage.clone();
                                break;
                            }
                        }
                    }
                    _ => {}
                }
            }

            if in_text {
                println!();
            }
            print_usage(&last_usage);
        }
        println!("\n{DIM}  done{RESET}\n");
        return;
    }

    let mut lines = stdin.lock().lines();

    loop {
        print!("{BOLD}{GREEN}> {RESET}");
        io::stdout().flush().ok();

        let line = match lines.next() {
            Some(Ok(l)) => l,
            _ => break,
        };

        let input = line.trim();
        if input.is_empty() {
            continue;
        }

        match input {
            "/quit" | "/exit" => break,
            "/clear" => {
                agent = build_agent(
                    &provider_name,
                    &model,
                    &api_key,
                    base_url.as_deref(),
                    &skills,
                );
                println!("{DIM}  (conversation cleared){RESET}\n");
                continue;
            }
            s if s.starts_with("/model ") => {
                let new_model = s.trim_start_matches("/model ").trim();
                agent = build_agent(
                    &provider_name,
                    new_model,
                    &api_key,
                    base_url.as_deref(),
                    &skills,
                );
                println!("{DIM}  (switched to {new_model}, conversation cleared){RESET}\n");
                continue;
            }
            s if s.starts_with("/price ") => {
                let symbol = s.trim_start_matches("/price ").trim();
                if symbol.is_empty() {
                    println!("{DIM}  Usage: /price bitcoin  or  /price AAPL{RESET}\n");
                    continue;
                }
                println!("{DIM}  fetching {symbol}...{RESET}");
                let tool = tools::GetPriceTool::new();
                execute_tool_direct(&tool, serde_json::json!({"symbol": symbol})).await;
                continue;
            }
            "/market" => {
                println!("{DIM}  fetching market overview...{RESET}");
                let tool = tools::GetMarketOverviewTool::new();
                execute_tool_direct(&tool, serde_json::json!({})).await;
                continue;
            }
            s if s.starts_with("/search ") => {
                let query = s.trim_start_matches("/search ").trim();
                if query.is_empty() {
                    println!("{DIM}  Usage: /search bitcoin  or  /search apple{RESET}\n");
                    continue;
                }
                println!("{DIM}  searching for '{query}'...{RESET}");
                let tool = tools::SearchSymbolTool::new();
                execute_tool_direct(&tool, serde_json::json!({"query": query})).await;
                continue;
            }
            s if s.starts_with("/compare ") => {
                let args: Vec<&str> = s
                    .trim_start_matches("/compare ")
                    .split_whitespace()
                    .collect();
                if args.len() < 2 {
                    println!(
                        "{DIM}  Usage: /compare bitcoin ethereum  or  /compare AAPL MSFT{RESET}\n"
                    );
                    continue;
                }
                println!(
                    "{DIM}  comparing {} assets concurrently...{RESET}",
                    args.len()
                );
                let futures: Vec<_> = args
                    .iter()
                    .map(|symbol| {
                        let sym = symbol.to_string();
                        async move {
                            let tool = tools::GetPriceTool::new();
                            let ctx = yoagent::types::ToolContext {
                                tool_call_id: "direct".into(),
                                tool_name: "get_price".into(),
                                cancel: tokio_util::sync::CancellationToken::new(),
                                on_update: None,
                                on_progress: None,
                            };
                            let result =
                                tool.execute(serde_json::json!({"symbol": sym}), ctx).await;
                            (sym, result)
                        }
                    })
                    .collect();
                let results = futures::future::join_all(futures).await;
                println!();
                println!(
                    "{BOLD}{CYAN}  ┌─ Comparison ──────────────────────────────────────{RESET}"
                );
                for (sym, result) in &results {
                    match result {
                        Ok(r) => {
                            for c in &r.content {
                                if let yoagent::types::Content::Text { text } = c {
                                    // Indent each line
                                    for line in text.lines() {
                                        println!("{CYAN}  │{RESET} {line}");
                                    }
                                    println!("{CYAN}  │{RESET}");
                                }
                            }
                        }
                        Err(e) => println!("{CYAN}  │{RESET} {RED}{sym}: Error — {e}{RESET}"),
                    }
                }
                println!(
                    "{BOLD}{CYAN}  └────────────────────────────────────────────────────{RESET}\n"
                );
                continue;
            }
            s if s.starts_with("/history ")
                || s.starts_with("/ta ")
                || s.starts_with("/chart ") =>
            {
                let cmd = if s.starts_with("/history") {
                    "/history "
                } else if s.starts_with("/ta") {
                    "/ta "
                } else {
                    "/chart "
                };
                let parts: Vec<&str> = s.trim_start_matches(cmd).split_whitespace().collect();
                if parts.is_empty() {
                    println!("{DIM}  Usage: /history bitcoin [30d]  or  /ta AAPL 1y{RESET}\n");
                    continue;
                }
                let symbol = parts[0];
                let range = parts.get(1).copied().unwrap_or("30d");
                println!("{DIM}  fetching {symbol} history ({range})...{RESET}");
                let tool = tools::GetPriceHistoryTool::new();
                execute_tool_direct(&tool, serde_json::json!({"symbol": symbol, "range": range}))
                    .await;
                continue;
            }
            s if s.starts_with("/news") => {
                let query = s.trim_start_matches("/news").trim();
                if query.is_empty() {
                    println!("{DIM}  Usage: /news bitcoin  or  /news AAPL earnings{RESET}\n");
                    continue;
                }
                println!("{DIM}  fetching news for '{query}'...{RESET}");
                let tool = tools::GetNewsTool::new();
                execute_tool_direct(&tool, serde_json::json!({"query": query})).await;
                continue;
            }
            s if s.starts_with("/watchlist") || s.starts_with("/watch") || s.starts_with("/wl") => {
                handle_watchlist_command(s).await;
                continue;
            }
            s if s.starts_with("/portfolio") || s.starts_with("/pf") || s.starts_with("/trade") => {
                handle_portfolio_command(s).await;
                continue;
            }
            s if s.starts_with("/alert") => {
                handle_alert_command(s).await;
                continue;
            }
            "/help" | "/?" => {
                print_help();
                continue;
            }
            s if s.starts_with('/') && !s[1..].contains(char::is_whitespace) => {
                // Unknown single-word slash command
                println!(
                    "{DIM}  Unknown command: {s}. Type /help for available commands.{RESET}\n"
                );
                continue;
            }
            _ => {}
        }

        let mut rx = agent.prompt(input).await;
        let mut last_usage = Usage::default();
        let mut in_text = false;

        while let Some(event) = rx.recv().await {
            match event {
                AgentEvent::ToolExecutionStart {
                    tool_name, args, ..
                } => {
                    if in_text {
                        println!();
                        in_text = false;
                    }
                    let summary = format_tool_summary(&tool_name, &args);
                    print!("{YELLOW}  ▶ {summary}{RESET}");
                    io::stdout().flush().ok();
                }
                AgentEvent::ToolExecutionEnd { is_error, .. } => {
                    if is_error {
                        println!(" {RED}✗{RESET}");
                    } else {
                        println!(" {GREEN}✓{RESET}");
                    }
                }
                AgentEvent::MessageUpdate {
                    delta: StreamDelta::Text { delta },
                    ..
                } => {
                    if !in_text {
                        println!();
                        in_text = true;
                    }
                    print!("{}", delta);
                    io::stdout().flush().ok();
                }
                AgentEvent::AgentEnd { messages } => {
                    for msg in messages.iter().rev() {
                        if let AgentMessage::Llm(Message::Assistant { usage, .. }) = msg {
                            last_usage = usage.clone();
                            break;
                        }
                    }
                }
                _ => {}
            }
        }

        if in_text {
            println!();
        }
        print_usage(&last_usage);
        println!();
    }

    println!("\n{DIM}  bye 👋{RESET}\n");
}

/// Execute a tool directly and print its output. Used by slash commands.
async fn execute_tool_direct(tool: &dyn yoagent::types::AgentTool, params: serde_json::Value) {
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
async fn handle_watchlist_command(input: &str) {
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
async fn handle_portfolio_command(input: &str) {
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
                    println!("{DIM}  Cash remaining: ${:.2}{RESET}\n", portfolio.cash);
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
        Some("history") | Some("log") | Some("trades") => {
            let portfolio = tools::portfolio::Portfolio::load();
            let limit = if parts.len() >= 2 {
                parts[1].parse::<usize>().unwrap_or(20)
            } else {
                20
            };
            println!("\n{}", portfolio.history_report(limit));
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

                // Check for stop-loss / take-profit triggers
                let triggered = portfolio.check_stop_loss_take_profit(&price_map);
                if !triggered.is_empty() {
                    let mut portfolio_mut = portfolio.clone();
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
                    println!("\n{}", portfolio.summary_with_prices(&price_map));
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
async fn fetch_live_price_for_trade(symbol: &str) -> Option<f64> {
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
async fn handle_alert_command(input: &str) {
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
                        let emoji = if *condition == "above" { "📈" } else { "📉" };
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

fn print_help() {
    println!("\n{BOLD}{CYAN}  yoyo commands{RESET}");
    println!("{DIM}  ─────────────────────────────────────────{RESET}");
    println!(
        "  {BOLD}/price{RESET} <symbol>      Quick price check (e.g. /price bitcoin, /price AAPL)"
    );
    println!("  {BOLD}/history{RESET} <sym> [rng]  Price history + TA with chart (e.g. /history bitcoin 30d)");
    println!("  {BOLD}/ta{RESET} <sym> [rng]       Alias for /history (e.g. /ta AAPL 90d)");
    println!("  {BOLD}/market{RESET}              Market overview — top crypto + US indices");
    println!("  {BOLD}/news{RESET} <query>        Latest news headlines (e.g. /news bitcoin, /news AAPL earnings)");
    println!("  {BOLD}/search{RESET} <query>      Find a symbol by name or ticker");
    println!("  {BOLD}/compare{RESET} <a> <b>     Compare two assets side by side");
    println!("  {BOLD}/watchlist{RESET}           Show your watchlist with current prices");
    println!("  {BOLD}/wl + {RESET}<symbol>       Add to watchlist (shorthand: /wl + bitcoin)");
    println!("  {BOLD}/wl - {RESET}<symbol>       Remove from watchlist");
    println!("  {BOLD}/portfolio{RESET}           Paper trading portfolio summary");
    println!(
        "  {BOLD}/pf buy{RESET} <sym> <qty> [price] [reason]  Open a buy (auto-fetches price!)"
    );
    println!(
        "  {BOLD}/pf sell{RESET} <sym> <qty> [price] [reason] Open a short (auto-fetches price!)"
    );
    println!(
        "  {BOLD}/pf close{RESET} <id> [price]       Close position (auto-fetches if omitted)"
    );
    println!("  {BOLD}/pf sl{RESET} <id> <price>         Set stop-loss on a trade");
    println!("  {BOLD}/pf tp{RESET} <id> <price>         Set take-profit on a trade");
    println!(
        "  {BOLD}/pf history{RESET} [N]            Show trade history (last N trades, default: 20)"
    );
    println!("  {BOLD}/pf reset{RESET}            Reset portfolio to $100K");
    println!("  {BOLD}/alert{RESET}               Show price alerts + check for triggers");
    println!("  {BOLD}/alert{RESET} <sym> above/below <price> [note]  Set a price alert");
    println!("  {BOLD}/alert rm{RESET} <id>        Remove an alert");
    println!("  {BOLD}/alert clear{RESET}          Clear triggered alerts");
    println!("  {BOLD}/clear{RESET}               Clear conversation history");
    println!("  {BOLD}/model{RESET} <name>        Switch to a different model");
    println!("  {BOLD}/help{RESET}                Show this help");
    println!("  {BOLD}/quit{RESET}                Exit yoyo");
    println!();
    println!("{DIM}  Ranges for /history: 1d, 7d, 30d, 90d, 1y (default: 30d){RESET}");
    println!("{DIM}  Or just type naturally: \"What's BTC done over the last month?\"{RESET}\n");
}

fn build_agent(
    provider: &str,
    model: &str,
    api_key: &str,
    base_url: Option<&str>,
    skills: &SkillSet,
) -> Agent {
    // Combine default coding tools with custom trading tools
    let mut all_tools = default_tools();
    all_tools.extend(tools::trading_tools());

    match provider {
        "anthropic" => Agent::new(AnthropicProvider)
            .with_system_prompt(SYSTEM_PROMPT)
            .with_model(model)
            .with_api_key(api_key)
            .with_skills(skills.clone())
            .with_tools(all_tools),
        "apieasy" => {
            let url = base_url.unwrap_or("https://www.apieasy.ai");
            Agent::new(ProxyAnthropicProvider::new(url))
                .with_system_prompt(SYSTEM_PROMPT)
                .with_model(model)
                .with_api_key(api_key)
                .with_skills(skills.clone())
                .with_tools(all_tools)
        }
        _ => {
            // OpenAI-compatible providers (kimi, deepseek, openai, groq, etc.)
            let (default_base_url, compat) = match provider {
                "kimi" | "moonshot" => ("https://api.moonshot.cn/v1", OpenAiCompat::default()),
                "deepseek" => ("https://api.deepseek.com/v1", OpenAiCompat::deepseek()),
                "openai" => ("https://api.openai.com/v1", OpenAiCompat::openai()),
                "groq" => ("https://api.groq.com/openai/v1", OpenAiCompat::groq()),
                "openrouter" => ("https://openrouter.ai/api/v1", OpenAiCompat::openrouter()),
                _ => ("http://localhost:11434/v1", OpenAiCompat::default()),
            };

            let url = base_url.unwrap_or(default_base_url);
            let model_config = ModelConfig {
                id: model.into(),
                name: model.into(),
                api: provider::ApiProtocol::OpenAiCompletions,
                provider: provider.into(),
                base_url: url.into(),
                reasoning: false,
                context_window: 128_000,
                max_tokens: 4096,
                cost: Default::default(),
                headers: Default::default(),
                compat: Some(compat),
            };

            Agent::new(OpenAiCompatProvider)
                .with_system_prompt(SYSTEM_PROMPT)
                .with_model(model)
                .with_api_key(api_key)
                .with_model_config(model_config)
                .with_skills(skills.clone())
                .with_tools(all_tools)
        }
    }
}

fn format_tool_summary(tool_name: &str, args: &serde_json::Value) -> String {
    match tool_name {
        "bash" => {
            let cmd = args
                .get("command")
                .and_then(|v| v.as_str())
                .unwrap_or("...");
            format!("$ {}", truncate(cmd, 80))
        }
        "read_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("?");
            format!("read {}", path)
        }
        "write_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("?");
            format!("write {}", path)
        }
        "edit_file" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or("?");
            format!("edit {}", path)
        }
        "list_files" => {
            let path = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            format!("ls {}", path)
        }
        "search" => {
            let pat = args.get("pattern").and_then(|v| v.as_str()).unwrap_or("?");
            format!("search '{}'", truncate(pat, 60))
        }
        // Trading tools
        "get_price" => {
            let symbol = args.get("symbol").and_then(|v| v.as_str()).unwrap_or("?");
            format!("📈 price {}", symbol)
        }
        "search_symbol" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("?");
            format!("🔍 search '{}'", query)
        }
        "get_market_overview" => "🌍 market overview".to_string(),
        "get_price_history" => {
            let symbol = args.get("symbol").and_then(|v| v.as_str()).unwrap_or("?");
            let range = args.get("range").and_then(|v| v.as_str()).unwrap_or("30d");
            format!("📊 history {} ({})", symbol, range)
        }
        "get_news" => {
            let query = args.get("query").and_then(|v| v.as_str()).unwrap_or("?");
            format!("📰 news '{}'", query)
        }
        _ => tool_name.to_string(),
    }
}

fn truncate(s: &str, max: usize) -> &str {
    match s.char_indices().nth(max) {
        Some((idx, _)) => &s[..idx],
        None => s,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_short_string() {
        assert_eq!(truncate("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_exact_length() {
        assert_eq!(truncate("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_long_string() {
        assert_eq!(truncate("hello world", 5), "hello");
    }

    #[test]
    fn test_truncate_unicode() {
        assert_eq!(truncate("héllo wörld", 5), "héllo");
    }

    #[test]
    fn test_truncate_empty() {
        assert_eq!(truncate("", 5), "");
    }
}
