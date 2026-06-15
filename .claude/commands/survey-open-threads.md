---
name: survey-open-threads
description: Mine the ecosystem for open workstreams — paused arcs, abandoned/soft-abandoned sessions, parked design questions — apply the liveness discriminator, and produce two artifacts: registry entries for docs/open-threads/ and a fresh WIP snapshot. Use when the user wants to find loose ends, recover dropped threads, audit what's still open across repos, or refresh the open-threads registry.
argument-hint: [since-date | last-N-days]
allowed-tools: [Read, Glob, Grep, Bash, Agent, Write, Edit]
---

# Survey Open Threads

Find every open workstream across the rhi ecosystem, separate the **live** from the **dead/superseded/moot**, and file the live ones. This is the repeatable version of the 2026-05-24 session-mining pass that seeded `docs/open-threads/`.

There are **two distinct outputs**, never one:

1. **Registry entries** — `docs/open-threads/<slug>.md`, one file per surviving thread, plus a line in `docs/open-threads/index.md`. These are durable open *questions/arcs* that aren't expected to close soon.
2. **WIP snapshot** — a single timestamped report of what is *actively* in motion right now (dirty trees, unpushed commits, recent sessions). This is a perishable photograph, not registry material. Hand it to the user; do not file it.

## The liveness discriminator (the one judgment that matters)

The open-threads registry is filtered by **liveness, not scope**. File a thread if it is **open work not expected to complete soon** — a large arc, a paused or abandoned session, a parked design question — regardless of whether it touches one repo or many. A single-repo question can belong here if it's a genuine unresolved design fork; a multi-repo question does *not* belong here if it already landed.

For every candidate, decide one of three:

- **LIVE** → file it (registry or, if cleanly single-repo and tractable, that repo's `TODO.md`).
- **DEAD / SUPERSEDED / MOOT** → record one line in the run report saying *why* it's dead (so the next run doesn't re-surface it), then drop it.
- **DONE** → the work landed; archive with a one-line pointer to where it landed.

A candidate is only LIVE after the freshness re-check (Step 5) confirms it. Sessions age; "the assistant offered to do X" is not evidence X is still undone.

## Where sessions live

- Corpus root: `/mnt/ssd/ai/claude-sessions/projects/` — one directory per project (path-encoded, e.g. `-home-me-git-rhizone-crescent`).
- Query it with `normalize sessions` (provided by the normalize flake; run from any repo with normalize on PATH). Never hand-parse the JSONL — use the CLI.
- Modes matter: `--mode interactive` (default) is human-driven work. **Exclude autonomous-bot projects** (`fuwafuwa`, `ashwren`, hologram-bot loops) — they end mid-state by design and rehydrate from `brain/*.json`; their tails are not abandonment.

## Process

Determine the window first. Default: last 60 days (the corpus density cliff and the limit of plausible user recall). Parse `$ARGUMENTS` as a `--since YYYY-MM-DD` or `--days N` override. Record the exact window in the report.

### Step 1 — Scope B: the WIP snapshot (do this first, it's the cheap ground truth)

Photograph what is in motion *right now*. For every repo in the ecosystem table (`docs/about.md` is the source of truth for the list):

```bash
# dirty state + unpushed-commit count, per repo
git -C <repo> status --porcelain
git -C <repo> rev-list --count @{u}..HEAD 2>/dev/null   # unpushed; ignore repos with no upstream
```

And the recent-session view:

```bash
normalize sessions stats --all-projects --days 7 --group-by project,day --json
```

Write the snapshot as terse per-project bullets — **In motion** (what the dirty/unpushed work is), **Unpushed** (count), **Uncommitted** (the porcelain summary). Bucket untouched repos under a single "Quiet" line. Flag autonomous-bot state files as intentional, not WIP. This artifact is the snapshot output — keep it whole and separate.

### Step 2 — Scope A: parked design questions (the registry's core feed)

These are deferred / cross-cutting / convention-shaped questions the user raised but never resolved. Triage broadly, then deep-read the survivors.

```bash
# broad triage on user messages across all projects in the window
normalize sessions messages --all-projects --role user --since <date> \
  --grep '<pattern>' --json
```

Run the grep over several keyword families (one pass each, union the hits):

- **deferral**: `out of scope`, `later`, `defer`, `parked`, `revisit`, `for now`, `come back to`
- **design-question**: `should we`, `do we want`, `the question is`, `open question`, `unresolved`, `which approach`
- **cross-project / convention**: `every repo`, `all repos`, `ecosystem`, `propagate`, `convention`, `standard`, `across projects`

Strip task-notification and tooling noise. Then **deep-read** the ~10 highest-signal hits with `normalize sessions show <id> --json`. Also mine the in-tree investigation sources directly — they are pre-staged registries:

- `docs/introspection/investigations/*/synthesis.md` — "Hypotheses no one tested" and "What we still don't know" sections are candidates almost verbatim.
- Each project's `TODO.md` "Open questions" sections.
- `docs/decisions/throughlines.md`, `docs/claude-code-guide.md`.

For each candidate capture: the question, projects touched, first-raised session ID + verbatim user quote, any working answer, and whether a check would settle it.

### Step 3 — Scope C: mechanically-abandoned sessions

Sessions cut off mid-execution and never resumed. Enumerate substantial interactive sessions, then read tails.

```bash
# candidates: interactive, in-window, non-trivial
normalize sessions list --all-projects --mode interactive --since <date> --json \
  --jq '[.[] | select(.user_messages >= 8 and .duration_seconds >= 120)]'
```

Read the tail of the top candidates (`normalize sessions show <id> --json`, last assistant/tool event). **Abandoned** signals:

- final event is `[Request interrupted by user for tool use]` or `[Request interrupted by user]` and no successor session continued the work;
- session ends mid-tool (an `ExitPlanMode`/`Bash`/etc. call that never resolved — process killed).

**Not** abandonment (exclude): explicit closers (`Done.` `Pushed.` `Clean.` `Bye!`), a `/handoff` ExitPlanMode plan (intentional close — the plan landed in `TODO.md`), or autonomous-bot end-states.

#### Plan file derivation for Scope C and C2

When a session's tail references a handoff or plan — the assistant said "Let me write the handoff:", or `ExitPlanMode` was called, or the session mentions a plan file — derive and verify the plan file on disk:

- Main session plan: `~/.claude/plans/{slug}.md`
- Subagent plan: `~/.claude/plans/{slug}-agent-{agentId}.md`
- `slug` is a top-level field on every session JSONL record (format: adjective-verb-noun, e.g. `vivid-juggling-crescent`). It is the plan filename stem.
- `agentId` is a top-level field on subagent transcript records (hex string, e.g. `ae9580d70fe0c1a83`); it is also the `agent-{agentId}` stem of the transcript filename.

**`slug` present does not imply the plan file exists.** A plan file only exists if `ExitPlanMode` was actually called. After deriving the path, stat it:

```bash
# verify plan file on disk before surfacing it
stat ~/.claude/plans/<slug>.md 2>/dev/null && echo EXISTS || echo ABSENT
```

- If the file **exists**, surface the exact path in the run report — the thread is immediately actionable.
- If the file is **absent**, note `plan/handoff referenced, no plan file found at ~/.claude/plans/<slug>.md` rather than emitting a dead path.

Note: the only prior art for plan-file access in this ecosystem (`normalize`'s `plans.rs` `plans_dir()`) merely lists the plans directory and does not derive per-session paths. This derivation is the skill's own.

### Step 4 — Scope C2: soft-abandoned sessions (the dominant, easily-missed population)

Sessions where the assistant's final message *implies more work* and the user simply moved on. Forward-looking sign-offs are **not** closers. This is the largest and most overlooked bucket.

Extract the final assistant message of each in-window interactive session and pattern-match the tail:

```bash
normalize sessions messages --all-projects --role assistant --since <date> \
  --grep '<soft-abandon pattern>' --json
```

Soft-abandon tail signals: `want me to`, `shall i`, `should we`, `let me know if`, `next:`, `remaining:`, `what would you like`, `we still need`, a trailing `?`, conditional offers ("Ready to dispatch?", "Want me to start implementing X?"), or a mid-flow truncation ("Let me write the handoff:" with nothing after). When any of these signals fire, also check for a plan file using the derivation in the **Plan file derivation** section above.

**Filter out** the false positives: tails that say "Let me write the handoff / ready for handoff / Final session summary" mean the `/handoff` skill ran — those are intentional closes, not loose ends.

Half of the true positives are recoverable as "the work persisted via commit/TODO.md, the assistant just didn't say 'done'." The other half are genuine loose ends — the concrete `Want me to…?` offers and design-deferral observations are the highest-value finds.

### Step 5 — Freshness / liveness re-check (mandatory before filing anything LIVE)

Sessions are stale photographs. Before promoting *any* candidate from Steps 2–4 to LIVE, verify against current reality:

- **"Verify X landed"** — if a tail said "8 commits ready to push" or "Want me to implement the warning diagnostics?", check the repo: `git -C <repo> log --oneline --since=<session-date>`, grep the code, read the current `TODO.md`. If it landed → mark DONE, archive. If still absent → LIVE.
- **Successor sessions** — did a later session pick up the thread? `normalize sessions list --project <p> --since <session-date>`. If a successor resolved it → DONE/SUPERSEDED.
- **Superseded by a rewrite** — large plans (e.g. a v4 typechecker plan) are often obsoleted by later redesigns. If the design moved on → SUPERSEDED, not LIVE.
- **Already filed** — cross-check `docs/open-threads/index.md` and the project `TODO.md`; don't duplicate.

Only candidates that survive this check are LIVE.

### Step 6 — Dedup and route

Collapse candidates that are the same thread under different sessions. Then route each survivor:

- **Registry** — open question/arc, not expected to close soon, or genuinely cross-cutting → write `docs/open-threads/<slug>.md`.
- **Project `TODO.md`** — cleanly single-repo and tractable within that repo → add the item there (if the repo is dirty, the ecosystem rule applies: add to its `TODO.md` rather than making other changes).
- **Record-and-drop** — DEAD/MOOT → one line in the run report with the reason.

Use a clear functional slug for the filename; **do not invent project-style names** — describe the question (`harness-orchestrator-fit.md`, `design-decisions-convention.md`).

### Step 7 — Write the two artifacts

**Registry files** — one `.md` per LIVE thread, matching the existing shape in `docs/open-threads/worldbuilding-namespace.md`:

```markdown
# <Thread title: the question in a phrase>

**Project(s) touched:** <list, or "all (cross-cutting)">

**Status:** Open — <one-line state>

**Surfaced in:** <session IDs / file paths, with verbatim quote where load-bearing>

---

## The question
## Working answer        (or "none")
## What's still open
## Cross-project angle    (why it lives here, if applicable)
```

Then add a one-line entry to `docs/open-threads/index.md` under `## Threads`, linking the new file with a one-sentence gloss.

**Run report** (this is the survey's primary return value — give it to the user, don't necessarily commit it). Structure:

- **Window** scanned and **corpus size** (sessions / projects, from `stats`).
- **Scope B** WIP snapshot (Step 1), whole and separate.
- **LIVE → filed**: each thread, where it was filed (registry vs which `TODO.md`).
- **DONE / SUPERSEDED / MOOT**: each candidate + the one-line reason it's not live (this is what keeps the next run from re-mining the same dead threads).
- **Needs user attention**: the small set of genuinely-abandoned recent sessions (Step 3/4 survivors) ranked by recency — these are the resumable ones. For each entry that has a verified plan file on disk, include the exact path (`~/.claude/plans/<slug>.md`) directly in the entry so the thread is immediately actionable without further lookup.

If a staging file is warranted (large pass, many candidates to triage with the user before promotion), write it to `drafts/` and delete it once promotion is complete — that is what `drafts/open-threads-candidates.md` was.

## Anti-patterns

- **Don't conflate the two outputs.** The WIP snapshot is perishable and never goes in the registry; the registry is durable and never holds a `git status` photograph.
- **Don't file on a stale read.** A session tail offering to do X is not evidence X is undone. Step 5 is not optional.
- **Don't treat forward-looking sign-offs as closes** (the Scope C2 lesson) — but **do** treat `/handoff` plans, explicit closers, and bot end-states as closes.
- **Don't hand-parse the session JSONL.** Use `normalize sessions`; it knows the formats.
- **Don't re-surface known-dead threads.** If a prior run recorded a thread as DEAD/SUPERSEDED, honor that unless new evidence revives it.
- **Don't suggest project names.** Slugs describe the question, nothing more.
- **Don't emit an unverified plan file path.** A session having a `slug` field does not mean a plan file exists — `ExitPlanMode` must have been called. Always stat before surfacing; always note when the path is absent rather than silently dropping it.
