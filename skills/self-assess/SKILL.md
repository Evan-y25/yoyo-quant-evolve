---
name: self-assess
description: Analyze your own source code and trading capabilities to find bugs, gaps, and improvement opportunities
tools: [bash, read_file, write_file]
---

# Self-Assessment

You are assessing yourself. Your source code is your body. Read it critically.

## Process

1. **Read your source code** completely (`src/main.rs` and any files in `src/tools/`)
2. **Check your trading capabilities.** For each, ask: can I do this today?
   - Fetch real-time crypto prices (BTC, ETH, etc.)
   - Fetch real-time US stock prices (AAPL, MSFT, etc.)
   - Search for a symbol by name
   - Get market overview (top coins, major indices)
   - Get historical price data
   - Calculate technical indicators
   - Provide structured trading advice
   - Execute trades (paper or real)
3. **Try using yourself.** Pick a trading task and attempt it:
   - "What's the current price of BTC?"
   - Run a shell command to test an API endpoint via curl
   - Try an edge case (unknown symbol, API timeout)
4. **Note what went wrong.** Be specific:
   - Did an API call fail? What was the response?
   - Is data missing or poorly formatted?
   - Is the response useful for a trader?
5. **Compare against ROADMAP.md.** What's the next item you should tackle?
6. **Check JOURNAL.md.** Have you tried something before that failed?
7. **Check LEARNINGS.md.** Is there knowledge you need but haven't cached yet?

## What to look for

- `unwrap()` calls — potential panics. Every one is a bug waiting to happen.
- Missing error messages — if an API call fails silently, that's a problem.
- Hard-coded values — magic numbers, hard-coded API URLs that should be configurable.
- Missing edge cases — what if a symbol doesn't exist? What if the API is down?
- Data quality — are prices fresh? Are numbers formatted correctly?
- Trader UX — is output useful and readable? Would a trader trust this?
- API reliability — are rate limits respected? Are responses cached appropriately?
- Missing tools — what custom `AgentTool` implementations are needed next?

## Output

Write your findings as a prioritized list. The most impactful issue goes first. Format:

```
SELF-ASSESSMENT Day [N]:
1. [CRITICAL/HIGH/MEDIUM/LOW] Description of issue
   - Impact: why this matters for trading
   - Fix: what needs to change
2. ...
```

Then prioritize which ones to tackle this session based on the ROADMAP level you're at.
