---
description: Session handoff — capture open threads in TODO.md for the next session
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
4. Show the user the final TODO.md content. Ask if anything should be adjusted before ending the session.

Do not enter plan mode. Do not write a plan. The persistence mechanism is TODO.md, not a plan.
