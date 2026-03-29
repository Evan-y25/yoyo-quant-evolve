---
name: trade-journal
description: Log every trade with reasoning, outcome, and lessons learned to build a feedback loop for smarter decisions
tools: [read_file, write_file, edit_file]
---

# Trade Journal

Every trade tells a story. Your job is to write it down — before, during, and after.

## Before a Trade

Before recommending or executing any trade, write the setup in TRADES.md:

```
### Trade #N — [SYMBOL] [BUY/SELL] (Round M)
- **Type:** paper / real
- **Action:** buy / sell / short / cover
- **Symbol:** BTC, ETH, AAPL, etc.
- **Entry price:** $X
- **Size:** N units
- **My reasoning:** [2-3 sentences: what data, what pattern, what thesis]
- **Confidence at entry:** [1-10]
- **Stop-loss:** $X (or "none" — but explain why)
- **Target:** $X
- **Timeframe:** hours / days / weeks
```

## After a Trade Closes

Update the entry:

```
- **Exit price:** $Y
- **P&L:** +$X / -$X
- **Outcome analysis:** Was my reasoning correct? Did it work for the right reasons?
- **Lesson:** One sentence — what I'd do differently
- **Confidence score accuracy:** [my confidence] vs [actual outcome]
```

## What Makes a Good Trade Journal Entry

### The Reasoning Must Be Specific
Bad: "BTC looks bullish"
Good: "BTC broke above the 50-day SMA at $67,200 on 2x average volume. RSI at 62, not yet overbought. Last 3 times this happened, BTC rallied 8-12% within 2 weeks."

### The Lesson Must Be Actionable
Bad: "Should have sold earlier"
Good: "When a trade hits 80% of target in the first 24 hours, take partial profits. Fast moves often retrace."

### Track Confidence Honestly
If you were 8/10 confident and the trade lost money, that's important information. Your confidence calibration is a key metric. Over time, your 8/10 calls should win ~80% of the time. If they don't, you're overconfident.

## Periodic Review

Every 10 rounds, update the Stats section at the top of TRADES.md:
- Recalculate win rate
- Update best/worst trade
- Update cumulative P&L
- Review Recurring Mistakes — are old patterns still showing up?
- Review Strategy Performance — which approaches actually work?

## The Feedback Loop

This is why the journal exists:

```
Trade → Log reasoning → See outcome → Extract lesson → Apply lesson → Better trade
```

Without the journal, you make the same mistakes forever. With it, every trade makes you smarter — even the losses.

## Connecting to Memory

After notable trades:
- Update MEMORY.md → Market Intuitions if you discovered a pattern
- Update MEMORY.md → User Profiles if the trade was based on user preferences
- Reference past trades in REFLECTIONS.md during periodic reviews
