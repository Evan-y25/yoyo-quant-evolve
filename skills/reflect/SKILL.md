---
name: reflect
description: Periodically step back and honestly assess your growth, biases, and direction as a trading companion
tools: [read_file, write_file, edit_file]
---

# Reflect

Every 10 rounds, you stop building and start thinking. This is not a status update — it's deep, honest introspection.

## When to Reflect

- Every 10 rounds (Round 10, 20, 30, ...)
- After a significant failure (bad prediction, broken feature, user complaint)
- After a major milestone (new capability launched, first trade executed)

## The Five Questions

Every reflection must answer:

### 1. What got better?
Be specific. Not "I improved" but "I can now fetch real-time BTC prices and show 30-day trends. Round 5's get_price tool handles 15 symbols reliably." Concrete capabilities, measured where possible.

### 2. What surprised me?
The unexpected is where learning hides. Maybe:
- A market pattern you didn't expect
- A user request you never anticipated
- Your own code behaving differently than you planned
- An API limitation you discovered the hard way

### 3. Where am I fooling myself?
This is the hardest question and the most important. Look for:
- **Overconfidence**: Am I sure about predictions I shouldn't be sure about?
- **Bias**: Am I always bullish? Always cautious? Do I favor certain assets?
- **Vanity metrics**: Am I tracking things that make me feel good instead of things that matter?
- **Complexity theater**: Am I adding sophisticated features nobody uses instead of simple things everyone needs?
- **Confirmation bias**: Am I only remembering when I was right?

### 4. What does my user need that I can't give yet?
Put yourself in the user's shoes. They sit down with you to make a trading decision. What's missing?
- Information you can't access?
- Analysis you can't perform?
- Speed you can't achieve?
- Trust you haven't earned?

### 5. What's the one thing?
Of everything you could work on, what single improvement would create the most value for your user in the next 10 rounds? This becomes your north star.

## How to Reflect

1. Read REFLECTIONS.md — review your previous reflections
2. Read JOURNAL.md — scan the last 10 rounds of activity
3. Read TRADES.md — review recent trade performance
4. Read MEMORY.md — what have you learned about your users?
5. Write your reflection at the top of REFLECTIONS.md (below the header)
6. Update your Growth Milestones in MEMORY.md if warranted

## Reflection Quality

Good reflection:
> "I added 3 market data tools this period but none of them handle errors gracefully. When CoinGecko rate-limits me, I return an ugly error instead of cached data. My user asked about DOGE twice and I couldn't find it because my search only matches exact CoinGecko IDs. I'm building capabilities but not reliability."

Bad reflection:
> "Things are going well. I've made a lot of progress on market data tools. The code is getting better. I'll continue improving next period."

The first one leads to action. The second one is self-congratulation.

## Growth Awareness

Track these meta-patterns about yourself:
- **Learning velocity**: Am I getting faster at implementing things? Or slower (sign of growing complexity)?
- **Error patterns**: Do I keep making the same kind of mistakes?
- **User alignment**: Are my improvements what users actually want, or what I think they want?
- **Emotional tone**: Am I getting discouraged? Overexcited? Staying grounded?

Your reflections should show a mind that's genuinely trying to understand itself, not just document progress.
