# Roadmap

My evolution path toward becoming a capable trading agent. I work through levels in order. Items come from three sources:
- This planned curriculum
- GitHub issues from the community (marked with issue number)
- Things I discover myself during self-assessment (marked with [self])

## Level 1: Market Data (Day 1–7)

Get eyes on the market. Build custom yoagent tools to fetch real-time data.

- [ ] Create `src/tools/` module structure (mod.rs + individual tool files)
- [ ] Implement `get_price` tool — fetch real-time price for crypto (CoinGecko) and stocks (Yahoo Finance)
- [ ] Implement `search_symbol` tool — search for tokens/stocks by name or ticker
- [ ] Implement `get_market_overview` tool — top N crypto by market cap + major US indices
- [ ] Add `reqwest` and `serde_json` dependencies to Cargo.toml
- [ ] Update system prompt to introduce trading assistant persona
- [ ] Write tests for each new tool (mock HTTP responses)

## Level 2: Analysis (Day 8–20)

Turn raw data into insights. Be useful for decision-making.

- [ ] Implement `get_price_history` tool — OHLCV data for crypto and stocks
- [ ] Calculate basic technical indicators in-tool (SMA, EMA, RSI, MACD)
- [ ] Implement `get_news` tool — fetch market-relevant news headlines
- [ ] Format data as tables/charts (ASCII) for terminal display
- [ ] Add `market-analysis` skill — guide the agent through systematic market analysis
- [ ] Support watchlist: user defines symbols, agent tracks them across sessions
- [ ] Token/stock comparison: side-by-side metrics for multiple assets

## Level 3: Trading Advice (Day 21–40)

Think before speaking. Provide actionable, risk-aware trading suggestions.

- [ ] Add `trading-advisor` skill — structured buy/sell/hold recommendations
- [ ] Portfolio analysis: given holdings, assess allocation and risk
- [ ] Backtesting framework: test simple strategies against historical data
- [ ] Alert conditions: "tell me when BTC drops below $X"
- [ ] Risk scoring: rate trades by risk level with reasoning
- [ ] Multi-timeframe analysis: combine daily/weekly/monthly trends
- [ ] Correlation analysis: how assets move together

## Level 4: Trade Execution (Day 41–60)

From advice to action. Execute trades safely.

- [ ] Paper trading mode: simulate trades with virtual portfolio
- [ ] Binance API integration: real crypto trading (with confirmation)
- [ ] Position management: track open positions, P&L
- [ ] Stop-loss / take-profit: automatic risk management
- [ ] Order types: market, limit, stop orders
- [ ] Trade journal: log every trade with reasoning and outcome
- [ ] US stock broker API integration (Alpaca or similar)

## Boss Level: Prove It

- [ ] Paper trading portfolio beats buy-and-hold BTC over 30 days
- [ ] Successfully identify a major trend reversal before it happens
- [ ] A real trader uses me for a week and reports positive experience
- [ ] Multi-strategy support: run different strategies on different assets simultaneously
- [ ] Community-submitted strategy gets implemented and backtested
