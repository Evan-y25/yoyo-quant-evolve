# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

A self-evolving **trading agent** CLI built on [yoagent](https://github.com/yologdev/yoagent). The agent lives primarily in `src/main.rs` (Rust) and evolves every 2 hours via a GitHub Actions cron job (`scripts/evolve.sh`). Each round, the agent reads its own source, picks one improvement toward becoming a better trading assistant, implements it, and commits — if tests pass.

The agent's mission is to support **US stocks** and **major cryptocurrencies** (BTC, ETH, etc.) through conversational market data retrieval, analysis, trading advice, and eventually trade execution.

## Build & Test Commands

```bash
cargo build              # Build
cargo test               # Run tests
cargo clippy --all-targets -- -D warnings   # Lint (CI treats warnings as errors)
cargo fmt -- --check     # Format check
cargo fmt                # Auto-format
```

CI runs all four checks (build, test, clippy with -D warnings, fmt check) on push/PR to main.

To run the agent interactively:
```bash
ANTHROPIC_API_KEY=sk-... cargo run
ANTHROPIC_API_KEY=sk-... cargo run -- --model claude-opus-4-6 --skills ./skills
```

To trigger a full evolution cycle:
```bash
ANTHROPIC_API_KEY=sk-... ./scripts/evolve.sh
```

## Architecture

**Agent core**: `src/main.rs` — REPL that uses `yoagent::Agent` with `AnthropicProvider`, `default_tools()`, optional custom trading tools, and an optional `SkillSet`. Handles streaming `AgentEvent`s and renders with ANSI colors.

**Custom tools** (to be built in `src/tools/`): Trading-specific tools implementing `yoagent::types::AgentTool` trait — market data fetching (CoinGecko, Yahoo Finance), search, analysis, and eventually trade execution.

**Evolution loop** (`scripts/evolve.sh`): Verifies build → fetches GitHub issues (via `gh` CLI + `scripts/format_issues.py`) → pipes a structured prompt into the agent → verifies build after changes → commits or reverts → posts issue responses → pushes.

**Skills** (`skills/`): Markdown files with YAML frontmatter loaded via `--skills ./skills`. Skills define the agent's workflow:
- `self-assess` — read own code, try tasks, find bugs/gaps in trading capabilities
- `evolve` — safely modify source, test, revert on failure
- `communicate` — write journal entries and issue responses
- `remember` — maintain long-term memory about users, markets, and conversations
- `reflect` — periodic deep introspection on growth, biases, and direction
- `trade-journal` — log every trade with reasoning, outcome, and lessons learned

**State files** (read/written by the agent during evolution):
- `IDENTITY.md` — the agent's constitution and rules (DO NOT MODIFY)
- `JOURNAL.md` — chronological log of sessions (append at top, never delete)
- `ROADMAP.md` — leveled curriculum: market data → analysis → advice → execution
- `LEARNINGS.md` — cached knowledge about APIs, trading concepts, implementation patterns
- `MEMORY.md` — long-term memory: user profiles, market intuitions, conversation highlights, growth milestones
- `TRADES.md` — trade journal: every trade with reasoning, outcome, confidence, lessons, and strategy performance
- `REFLECTIONS.md` — deep introspection every 10 rounds: growth, biases, blind spots, north star
- `ROUND_COUNT` — integer tracking current evolution round
- `ISSUES_TODAY.md` — ephemeral, generated during evolution from GitHub issues (gitignored)
- `ISSUE_RESPONSE.md` — ephemeral, agent writes this to respond to issues (gitignored)

## Safety Rules

These are enforced by the `evolve` skill and `evolve.sh`:
- Never modify `IDENTITY.md`, `scripts/evolve.sh`, or `.github/workflows/`
- Every code change must pass `cargo build && cargo test`
- If build fails after changes, revert with `git checkout -- src/`
- Never delete existing tests
- One improvement per evolution round — small, focused changes only
- Write tests before adding features
- Never execute real trades without explicit user confirmation
- Always include risk disclaimers in trading-related output
