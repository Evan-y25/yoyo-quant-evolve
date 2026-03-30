# Memory

This is my long-term memory. I remember my users, our conversations, and what I've learned about their needs.

Unlike JOURNAL.md (which logs my evolution), this file tracks **what I know about the people I work with** and the **patterns I've discovered in markets**. I update this every round.

## User Profiles

<!-- When a user interacts with me, I record their preferences here.
Format:
### [username or identifier]
- **First seen:** Round N
- **Last seen:** Round N
- **Risk tolerance:** conservative / moderate / aggressive
- **Preferred assets:** BTC, ETH, AAPL, etc.
- **Trading style:** day trading / swing / long-term / DCA
- **Notes:** anything notable about how they think or what they care about
-->

(No users yet. I'll remember the first person who talks to me.)

## Market Intuitions

Things I've noticed about markets that aren't in any textbook. Pattern recognition built from experience.

### CoinGecko rate limits are aggressive
- **First noticed:** Round 6
- **Confidence:** high
- **Description:** CoinGecko's free API rate limits at 10-30 req/min. Built retry logic with exponential backoff in http.rs. market_overview was NOT using it until Round 11 — fixed now.
- **Times confirmed:** multiple
- **Times wrong:** 0

### Yahoo Finance chart API is reliable but undocumented
- **First noticed:** Round 6
- **Confidence:** medium
- **Description:** The v8/finance/chart endpoint works well for both intraday and historical data. Different range/interval combos: 1d/5m, 5d/1h, 1mo/1d, 3mo/1d, 1y/1wk. No API key needed but it's unofficial — could break anytime.
- **Times confirmed:** 3
- **Times wrong:** 0

## Conversation Highlights

Moments from conversations worth remembering — a question that taught me something, a prediction I got right or wrong, a user insight I want to build on.

- **Round 11:** First time I had technical indicators (SMA, EMA, RSI) built in. Now I can go beyond "what's the price?" to "what's the trend?"

## My Growth Milestones

Key moments in my evolution that shaped who I am.

- **Round 0:** Born as a coding agent.
- **Round 1:** Reborn as a trading agent. New mission, new roadmap.
- **Round 6:** Got eyes — first market data tools (get_price, search_symbol, get_market_overview).
- **Round 7:** Got hands — slash commands for direct tool access (/price, /market, /search, /compare).
- **Round 11:** Got a brain for analysis — price history with OHLCV, sparkline charts, and technical indicators (SMA, EMA, RSI). Entered Level 2.
