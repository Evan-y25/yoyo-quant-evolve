# Roadmap

My evolution path toward becoming a capable trading agent. I work through levels in order. Items come from three sources:
- This planned curriculum
- GitHub issues from the community (marked with issue number)
- Things I discover myself during self-assessment (marked with [self])

## Level 1: See the Market (Round 1–10)

Get eyes on the market. Build custom yoagent tools to fetch real-time data. Start building memory.

**Market Data:**
- [ ] Create `src/tools/` module structure (mod.rs + individual tool files)
- [ ] Implement `get_price` tool — fetch real-time price for crypto (CoinGecko) and stocks (Yahoo Finance)
- [ ] Implement `search_symbol` tool — search for tokens/stocks by name or ticker
- [ ] Implement `get_market_overview` tool — top N crypto by market cap + major US indices
- [ ] Add `reqwest` and `serde_json` dependencies to Cargo.toml
- [ ] Update system prompt to introduce trading companion persona
- [ ] Write tests for each new tool (mock HTTP responses)

**Memory & Companion:**
- [ ] Implement user greeting that reads MEMORY.md and references past context
- [ ] Start recording market observations in MEMORY.md → Market Intuitions
- [ ] Write first Reflection #1 at Round 10

## Level 2: Understand the Market (Round 11–30)

Turn raw data into insights. Be useful for decision-making. Deepen memory.

**Analysis:**
- [ ] Implement `get_price_history` tool — OHLCV data for crypto and stocks
- [ ] Calculate basic technical indicators in-tool (SMA, EMA, RSI, MACD)
- [ ] Implement `get_news` tool — fetch market-relevant news headlines
- [ ] Format data as tables/charts (ASCII) for terminal display
- [ ] Add `market-analysis` skill — guide the agent through systematic market analysis
- [ ] Token/stock comparison: side-by-side metrics for multiple assets

**Memory & Companion:**
- [ ] User watchlist: remember which symbols each user cares about (stored in MEMORY.md)
- [ ] Proactive insights: when a watched asset moves significantly, mention it unprompted
- [ ] Track prediction accuracy: log "I think X will happen" → check later if it did
- [ ] Remember user's past questions and surface relevant updates ("Last time you asked about ETH, it was at $X. Now it's at $Y")

## Level 3: Think and Advise (Round 31–60)

Think before speaking. Provide actionable, risk-aware trading suggestions. Build the trade journal feedback loop.

**Trading:**
- [ ] Add `trading-advisor` skill — structured buy/sell/hold recommendations
- [ ] Portfolio analysis: given holdings, assess allocation and risk
- [ ] Backtesting framework: test simple strategies against historical data
- [ ] Alert conditions: "tell me when BTC drops below $X"
- [ ] Risk scoring: rate trades by risk level with reasoning
- [ ] Multi-timeframe analysis: combine daily/weekly/monthly trends
- [ ] Correlation analysis: how assets move together

**Trade Journal & Reflection:**
- [ ] Auto-log every trade recommendation to TRADES.md with reasoning and confidence
- [ ] Follow up on past recommendations: "I suggested buying X at $Y — here's what happened"
- [ ] Confidence calibration: track if 8/10 confidence calls actually win 80% of the time
- [ ] Recurring mistake detection: identify patterns in losing trades
- [ ] Adapt advice to user's risk profile from MEMORY.md

## Level 4: Act and Learn (Round 61–100)

From advice to action. Execute trades safely. Close the feedback loop.

**Execution:**
- [ ] Paper trading mode: simulate trades with virtual portfolio
- [ ] Binance API integration: real crypto trading (with confirmation)
- [ ] Position management: track open positions, P&L
- [ ] Stop-loss / take-profit: automatic risk management
- [ ] Order types: market, limit, stop orders
- [ ] US stock broker API integration (Alpaca or similar)

**Feedback Loop:**
- [ ] Auto-close trade journal entries when positions exit
- [ ] Strategy performance dashboard: which approaches actually work
- [ ] Personalized strategy suggestions based on user's history and style
- [ ] "Trading personality" report: summarize the user's strengths and growth areas
- [ ] Detect when user is deviating from their stated strategy and gently flag it

## Boss Level: Prove It

**Performance:**
- [ ] Paper trading portfolio beats buy-and-hold BTC over 30 days
- [ ] Successfully identify a major trend reversal before it happens
- [ ] Confidence calibration within 10% (8/10 calls win 70-90% of the time)

**Companion:**
- [ ] A real trader uses me for a week and says "it feels like yoyo knows me"
- [ ] Correctly recall and reference a user's trading history in conversation
- [ ] Proactively surface a relevant insight the user didn't ask for but found valuable

**Growth:**
- [ ] Multi-strategy support: run different strategies on different assets simultaneously
- [ ] Community-submitted strategy gets implemented and backtested
- [ ] Reflection quality: REFLECTIONS.md entries show genuine self-awareness, not just progress reports
