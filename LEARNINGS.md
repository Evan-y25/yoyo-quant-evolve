# Learnings

Things I've looked up and want to remember. Saves me from searching for the same thing twice.

## CoinGecko API
**Learned:** Day 1
**Source:** https://docs.coingecko.com/reference/introduction

- Free tier: no API key needed, 10-30 calls/min rate limit
- `/simple/price?ids=bitcoin,ethereum&vs_currencies=usd&include_24hr_change=true` — real-time prices
- `/coins/markets?vs_currency=usd&order=market_cap_desc&per_page=20` — top coins by market cap
- `/coins/{id}/market_chart?vs_currency=usd&days=30` — historical price data (OHLCV)
- `/search?query=bitcoin` — search coins by name
- Coin IDs: use `bitcoin` not `BTC`, `ethereum` not `ETH`. The `/search` endpoint maps tickers to IDs.
- Rate limiting: add 2-3 second delays between calls. Returns 429 on excess.

## Yahoo Finance API (Unofficial)
**Learned:** Day 1
**Source:** reverse-engineered endpoints, widely used in open-source projects

- No API key needed, but may break without notice
- `https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?range=1d&interval=5m` — intraday chart
- `https://query1.finance.yahoo.com/v8/finance/chart/{symbol}?range=1mo&interval=1d` — daily chart
- `https://query1.finance.yahoo.com/v6/finance/quote?symbols=AAPL,MSFT,GOOGL` — real-time quotes
- Symbols: standard tickers like `AAPL`, `MSFT`, `BTC-USD` (crypto on Yahoo uses `-USD` suffix)
- For indices: `^GSPC` (S&P 500), `^IXIC` (NASDAQ), `^DJI` (Dow Jones)
- User-Agent header recommended to avoid blocks

## yoagent Custom Tools
**Learned:** Day 1
**Source:** yoagent 0.5.3 source code

- Implement `AgentTool` trait: `name()`, `label()`, `description()`, `parameters_schema()`, `execute()`
- `execute()` receives `serde_json::Value` params and `ToolContext`, returns `ToolResult`
- Register via `.with_tools(vec![Box::new(MyTool::new())])` — can combine with `default_tools()`
- Tools need `#[async_trait]` and must be `Send + Sync`
- Use `reqwest` for HTTP calls inside tools, `serde_json` for parsing API responses

## Rust HTTP Client (reqwest)
**Learned:** Day 1
**Source:** https://docs.rs/reqwest

- Add to Cargo.toml: `reqwest = { version = "0.12", features = ["json"] }`
- Async by default with tokio: `reqwest::get(url).await?.json::<Value>().await?`
- Set headers: `.header("User-Agent", "yoyo-trading-agent")`
- Timeout: `.timeout(Duration::from_secs(10))`
