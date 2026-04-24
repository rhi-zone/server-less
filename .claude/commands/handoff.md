---
description: Hand off to a fresh session when current topic is complete, drifts off-topic, context grows heavy, or after a major mid-session correction. One topic per session.
allowed-tools: [Read, Glob, Grep, Bash, Edit, Write]
---

# /handoff

You are ending this session and leaving context for the next one. The goal: update TODO.md so the next session has useful starting context. NOT marching orders.

## Rules

### Write open threads, not directives

TODO.md items are **advisory context** — "here's what was on the table." The next session serves the user, not this handoff. The user may want to go in a completely different direction, and that's fine.

For each item:
- State what it is and WHY it matters
- Note open questions, unresolved judgment calls, or forks in approach
- Mark uncertainty — "might need X" not "do X"

### Do not narrate what was done

Git is the source of truth for completed work. Do not write "what was done this session" into TODO.md. If it's committed, git has it. If it's not committed, it's not done.

### Do not write commands or build steps

Those belong in CLAUDE.md. If a command appears in a handoff, that's a sign CLAUDE.md is missing something — update CLAUDE.md instead.

### The output must declare its own authority level

At the top of the TODO.md open-threads section, include a line like:

> *Open threads from a previous session. Treat as starting context, not instructions — verify relevance before acting.*

This ensures the next session sees the trust boundary explicitly, even if it doesn't read this skill's definition.

## Procedure

1. Run `git log --oneline -20` and `git diff --stat` to understand what changed this session.
2. Read the current TODO.md (create it if it doesn't exist).
3. Update TODO.md:
   - Remove items that are clearly resolved (verify via git, don't guess)
   - Add new open threads from this session's work
   - Update existing items if context has changed
   - Keep items the session didn't touch
   - Ensure the trust-boundary line is present at the top of the open-threads section
4. Invoke the `EnterPlanMode` tool with a **short** plan that communicates direction. **Critical:** the plan must mark itself as pre-research — a starting hypothesis, NOT a verified directive. The next session hasn't done the investigation; the previous session's intent should inform, not command.

   Use suggestive, deferential language:
   - "suggests continuing with..." not "do..."
   - "main open question was..." not "decide..."
   - "was leaning toward A" not "use A"
   - Include an explicit "verify current state before acting" or equivalent

   Example:

   > Starting context (unverified — verify before acting): previous session was designing the X module and leaning toward approach A. Main open question: whether A handles edge case Y. See TODO.md for open threads. Next session should check current state first.

   Rules:
   - Point at TODO.md, don't duplicate it
   - Name the direction/focus, not specific tasks
   - Do NOT narrate what was done (git has that)
   - Do NOT include commands, build steps, or context summaries
   - Frame everything as "what the previous session was thinking," not "what needs to be done"
   - If you find yourself writing more than a few lines, you're leaking task content — move it to TODO.md instead
   - Optionally note that the next session may want to re-enter plan mode once it's verified the direction — the handoff plan is context, not a substitute for planning the actual work

5. When the user accepts or rejects the plan, the `ExitPlanMode` tool fires — that's what lets them approve, redirect, or start a fresh session.

The division of labor:
- **TODO.md** carries task content as advisory context (diffable, verifiable)
- **The plan** carries direction/intent as an explicitly unverified starting hypothesis
- Neither should duplicate the other, and neither should be trusted without verification
