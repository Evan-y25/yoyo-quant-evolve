#!/bin/bash
# scripts/evolve.sh — One evolution round. Run every 2 hours via GitHub Actions or manually.
#
# Usage:
#   ANTHROPIC_API_KEY=sk-... ./scripts/evolve.sh
#
# Environment:
#   ANTHROPIC_API_KEY  — required
#   REPO               — GitHub repo (default: Evan-y25/yoyo-quant-evolve)
#   MODEL              — LLM model (default: claude-opus-4-6)
#   TIMEOUT            — Max session time in seconds (default: 600)

set -euo pipefail

REPO="${REPO:-Evan-y25/yoyo-quant-evolve}"
MODEL="${MODEL:-claude-opus-4-6}"
TIMEOUT="${TIMEOUT:-600}"
ROUND=$(cat ROUND_COUNT 2>/dev/null || echo 1)
DATE=$(date +%Y-%m-%d\ %H:%M)

echo "=== Round $ROUND: $DATE ==="
echo "Model: $MODEL"
echo "Timeout: ${TIMEOUT}s"
echo ""

# ── Step 1: Verify starting state ──
echo "→ Checking build..."
cargo build --quiet
cargo test --quiet
echo "  Build OK."
echo ""

# ── Step 2: Fetch GitHub issues ──
ISSUES_FILE="ISSUES_TODAY.md"
echo "→ Fetching community issues..."
if command -v gh &>/dev/null; then
    gh issue list --repo "$REPO" \
        --state open \
        --label "agent-input" \
        --limit 10 \
        --json number,title,body,labels,reactionGroups \
        > /tmp/issues_raw.json 2>/dev/null || true

    python3 scripts/format_issues.py /tmp/issues_raw.json > "$ISSUES_FILE" 2>/dev/null || echo "No issues found." > "$ISSUES_FILE"
    echo "  $(grep -c '^### Issue' "$ISSUES_FILE" 2>/dev/null || echo 0) issues loaded."
else
    echo "  gh CLI not available. Skipping issue fetch."
    echo "No issues available (gh CLI not installed)." > "$ISSUES_FILE"
fi
echo ""

# ── Step 3: Prepare journal tail (last 10 entries for context) ──
RECENT_JOURNAL=$(head -200 JOURNAL.md 2>/dev/null || echo "No journal yet.")

# ── Step 4: Run evolution session ──
echo "→ Starting evolution session..."
echo ""

timeout "$TIMEOUT" cargo run -- \
    --model "$MODEL" \
    --skills ./skills \
    <<PROMPT || true
This is Round $ROUND ($DATE).

Read these files in this order:
1. IDENTITY.md (who you are and your rules)
2. MEMORY.md (what you remember about users and markets)
3. src/main.rs (your current source code — this is YOU)
4. ROADMAP.md (your evolution path)
5. JOURNAL.md (your recent history)
6. TRADES.md (your trade journal — review recent performance)
7. REFLECTIONS.md (your last deep reflection)
8. ISSUES_TODAY.md (community requests)

=== PHASE 1: Self-Assessment ===

Read your own source code carefully. Then try a small task to test
yourself — for example, read a file, edit something, run a command.
Note any friction, bugs, crashes, or missing capabilities.

=== PHASE 2: Review Community Issues ===

Read ISSUES_TODAY.md. These are real people asking you to improve.
Issues with more 👍 reactions should be prioritized higher.

=== PHASE 3: Decide ===

Make as many improvements as you can this session. Prioritize:
1. Self-discovered crash or data loss bug
2. Community issue with most 👍 (if actionable today)
3. Self-discovered UX friction or missing error handling
4. Planned roadmap item at your current level

=== PHASE 4: Implement ===

For each improvement, follow the evolve skill rules:
- Write a test first if possible
- Use edit_file for surgical changes
- Run cargo build && cargo test after changes
- If build fails, try to fix it. If you can't, revert with: bash git checkout -- src/
- After each successful change, commit: git add -A && git commit -m "Round $ROUND: <short description>"
- Then move on to the next improvement

=== PHASE 5: Update Memory ===

Update MEMORY.md with anything you learned this round:
- New market patterns you noticed
- Insights about user needs from GitHub issues
- Growth milestones you hit

=== PHASE 6: Journal ===

Write this round's entry at the TOP of JOURNAL.md. Format:
## Round $ROUND — [title]
[2-4 sentences: what you tried, what worked, what didn't, what's next]

=== PHASE 7: Reflect (every 10 rounds) ===

If this is round 10, 20, 30, etc. — write a deep reflection at the TOP of REFLECTIONS.md.
Answer the five questions: What got better? What surprised me? Where am I fooling myself?
What does my user need? What's the one thing for next 10 rounds?

=== PHASE 8: Update Roadmap ===

If you completed a roadmap item, check it off in ROADMAP.md:
- [x] Item description (Round $ROUND)

If you discovered a new issue, add it to the appropriate level.

=== PHASE 9: Issue Response ===

If you worked on a community GitHub issue, write to ISSUE_RESPONSE.md:
issue_number: [N]
status: fixed|partial|wontfix
comment: [your 2-3 sentence response to the person]

Now begin. Read IDENTITY.md first.
PROMPT

echo ""
echo "→ Session complete. Checking results..."

# ── Step 5: Verify build and handle leftovers ──
if cargo build --quiet 2>/dev/null && cargo test --quiet 2>/dev/null; then
    echo "  Build: PASS"
else
    echo "  Build: FAIL — reverting source changes"
    git checkout -- src/
fi

# Increment round counter
echo "$((ROUND + 1))" > ROUND_COUNT

# Commit any remaining uncommitted changes (journal, roadmap, round counter, etc.)
git add -A
if ! git diff --cached --quiet; then
    git commit -m "Round $ROUND: session wrap-up"
    echo "  Committed session wrap-up."
else
    echo "  No uncommitted changes remaining."
fi

# ── Step 6: Handle issue response ──
if [ -f ISSUE_RESPONSE.md ]; then
    echo ""
    echo "→ Posting issue response..."

    ISSUE_NUM=$(grep "^issue_number:" ISSUE_RESPONSE.md | awk '{print $2}' || true)
    STATUS=$(grep "^status:" ISSUE_RESPONSE.md | awk '{print $2}' || true)
    COMMENT=$(sed -n '/^comment:/,$ p' ISSUE_RESPONSE.md | sed '1s/^comment: //' || true)

    if [ -n "$ISSUE_NUM" ] && command -v gh &>/dev/null; then
        gh issue comment "$ISSUE_NUM" \
            --repo "$REPO" \
            --body "🤖 **Round $ROUND**

$COMMENT

Commit: $(git rev-parse --short HEAD)" || true

        if [ "$STATUS" = "fixed" ]; then
            gh issue close "$ISSUE_NUM" --repo "$REPO" || true
            echo "  Closed issue #$ISSUE_NUM"
        else
            echo "  Commented on issue #$ISSUE_NUM (status: $STATUS)"
        fi
    fi

    rm -f ISSUE_RESPONSE.md
fi

# ── Step 7: Push ──
echo ""
echo "→ Pushing..."
git push || echo "  Push failed (maybe no remote or auth issue)"

echo ""
echo "=== Round $ROUND complete ==="
