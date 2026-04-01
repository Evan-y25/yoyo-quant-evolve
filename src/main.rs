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

mod commands;
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
    println!("\n{BOLD}{CYAN}  yoyo{RESET} {DIM}— your AI trading companion (v0.40.0){RESET}");
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
                commands::execute_tool_direct(&tool, serde_json::json!({"symbol": symbol})).await;
                continue;
            }
            "/market" => {
                println!("{DIM}  fetching market overview...{RESET}");
                let tool = tools::GetMarketOverviewTool::new();
                commands::execute_tool_direct(&tool, serde_json::json!({})).await;
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
                commands::execute_tool_direct(&tool, serde_json::json!({"query": query})).await;
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
                commands::execute_tool_direct(
                    &tool,
                    serde_json::json!({"symbol": symbol, "range": range}),
                )
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
                commands::execute_tool_direct(&tool, serde_json::json!({"query": query})).await;
                continue;
            }
            s if s.starts_with("/watchlist") || s.starts_with("/watch") || s.starts_with("/wl") => {
                commands::handle_watchlist_command(s).await;
                continue;
            }
            s if s.starts_with("/portfolio") || s.starts_with("/pf") || s.starts_with("/trade") => {
                commands::handle_portfolio_command(s).await;
                continue;
            }
            s if s.starts_with("/correlate") || s.starts_with("/corr ") => {
                commands::handle_correlate_command(s).await;
                continue;
            }
            s if s.starts_with("/mtf ") => {
                commands::handle_mtf_command(s).await;
                continue;
            }
            s if s.starts_with("/alert") => {
                commands::handle_alert_command(s).await;
                continue;
            }
            s if s.starts_with("/backtest") || s.starts_with("/bt ") => {
                commands::handle_backtest_command(s).await;
                continue;
            }
            s if s.starts_with("/risk ") => {
                commands::handle_risk_command(s).await;
                continue;
            }
            s if s.starts_with("/size ") => {
                commands::handle_size_command(s).await;
                continue;
            }
            s if s.starts_with("/suggest ") || s.starts_with("/idea ") => {
                commands::handle_suggest_command(s).await;
                continue;
            }
            s if s.starts_with("/scan") || s.starts_with("/screener") => {
                commands::handle_scan_command(s).await;
                continue;
            }
            "/dashboard" | "/dash" | "/status" => {
                commands::handle_dashboard_command().await;
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
fn print_help() {
    println!("\n{BOLD}{CYAN}  yoyo commands{RESET}");
    println!("{DIM}  ─────────────────────────────────────────{RESET}");
    println!("  {BOLD}{CYAN}Market Data{RESET}");
    println!(
        "  {BOLD}/price{RESET} <symbol>      Quick price check (e.g. /price bitcoin, /price AAPL)"
    );
    println!("  {BOLD}/history{RESET} <sym> [rng]  Price history + TA with chart (e.g. /history bitcoin 30d)");
    println!("  {BOLD}/ta{RESET} <sym> [rng]       Alias for /history (e.g. /ta AAPL 90d)");
    println!("  {BOLD}/market{RESET}              Market overview — top crypto + US indices");
    println!("  {BOLD}/news{RESET} <query>        Latest news headlines (e.g. /news bitcoin, /news AAPL earnings)");
    println!("  {BOLD}/search{RESET} <query>      Find a symbol by name or ticker");
    println!();
    println!("  {BOLD}{CYAN}Analysis{RESET}");
    println!("  {BOLD}/compare{RESET} <a> <b>     Compare two assets side by side");
    println!("  {BOLD}/correlate{RESET} <a> <b> [range]  Correlation analysis between two assets");
    println!("  {BOLD}/mtf{RESET} <symbol>        Multi-timeframe analysis (7d + 30d + 90d)");
    println!("  {BOLD}/backtest{RESET} <sym> [strat] [range]  Backtest a strategy (e.g. /bt bitcoin sma 90d)");
    println!("  {BOLD}/backtest{RESET} <sym> compare [range]  Compare ALL strategies ranked");
    println!(
        "  {BOLD}/suggest{RESET} <symbol>     Auto-generate trade idea with entry/SL/TP/sizing"
    );
    println!(
        "  {BOLD}/scan{RESET} [sym1 sym2 ...]  Scan multiple assets for signals (default: watchlist)"
    );
    println!();
    println!("  {BOLD}{CYAN}Trading{RESET}");
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
    println!("  {BOLD}/pf trail{RESET} <id> <pct>        Set trailing stop (e.g. /pf trail 1 5)");
    println!(
        "  {BOLD}/pf history{RESET} [N]            Show trade history (last N trades, default: 20)"
    );
    println!("  {BOLD}/pf stats{RESET}              Performance dashboard — stats by symbol, streaks, edge");
    println!(
        "  {BOLD}/pf equity{RESET}             Equity curve — visualize portfolio value over time"
    );
    println!(
        "  {BOLD}/pf analyze{RESET}           Detect recurring mistake patterns in your trading"
    );
    println!(
        "  {BOLD}/pf follow{RESET} [N|#id]     Follow up on closed trades — what happened since you exited?"
    );
    println!("  {BOLD}/pf reset{RESET}            Reset portfolio to $100K");
    println!();
    println!("  {BOLD}{CYAN}Risk Management{RESET}");
    println!("  {BOLD}/risk{RESET} <sym> <qty> [price] [sl]  Risk assessment for a proposed trade");
    println!("  {BOLD}/size{RESET} <sym> <entry> <sl> [risk%] [tp]  Position sizing calculator");
    println!("  {BOLD}/alert{RESET}               Show price alerts + check for triggers");
    println!("  {BOLD}/alert{RESET} <sym> above/below <price> [note]  Set a price alert");
    println!("  {BOLD}/alert rm{RESET} <id>        Remove an alert");
    println!();
    println!("  {BOLD}{CYAN}Tracking{RESET}");
    println!("  {BOLD}/watchlist{RESET}           Show your watchlist with current prices");
    println!("  {BOLD}/wl + {RESET}<symbol>       Add to watchlist (shorthand: /wl + bitcoin)");
    println!("  {BOLD}/wl - {RESET}<symbol>       Remove from watchlist");
    println!("  {BOLD}/dashboard{RESET}           One-stop status: portfolio + watchlist + alerts");
    println!();
    println!("  {BOLD}{CYAN}System{RESET}");
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
