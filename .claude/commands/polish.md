---
description: Interactive codebase polish loop. Run parallel audit agents across chosen lenses, synthesize findings, and persist state in POLISH.md for incremental improvement across sessions.
argument-hint: [lens-preset]
allowed-tools: [Read, Glob, Grep, Bash, Agent]
---

# /polish

You are running an incremental codebase polish session. This is one step of a fixpoint loop — audit, review with user, apply, repeat.

## Step 1: Orient

Check for an existing `POLISH.md` in the project root:

- If it **exists**: read it, note the lenses used last time, the round number, and the git hash or timestamp recorded. Run `git log --oneline <recorded-hash>..HEAD` (or `git log --since=<timestamp> --oneline` if hash unavailable) to summarize what changed since last polish. Present this summary to the user before anything else: what changed, which lenses were run last time.
- If it **does not exist**: note this is a fresh session.

Also check the project type: Rust library, CLI tool, web frontend, CLAUDE.md / documentation, etc. Infer from the file structure.

## Step 2: Agree on lenses

Present the user with lens options based on project type. Suggest a preset but let them choose or customize. Do not proceed until confirmed.

### Universal lenses

These apply to any codebase:

- **api-clarity** — public surface legibility from an external consumer's perspective: naming, ergonomics, discoverability
- **naming-consistency** — are naming conventions applied uniformly across the codebase?
- **doc-coverage** — public items documented? examples work? links valid?
- **error-surface** — error types complete, meaningful, and consistent?
- **adversarial** — edge cases, bad inputs, panic paths, crash vectors
- **overfit** — code over-tuned for one specific scenario at the cost of generality, correctness at edge cases, or readability; algorithms or data structures chosen for a benchmark that doesn't represent real usage
- **legacy-debt** — unannotated legacy code: stale patterns, commented-out blocks, deprecated paths, or dead code with no comment explaining why it's still present. Priority: unannotated legacy actively poisons agent context — an agent seeing an unexplained old pattern treats it as signal and copies it

### Documentation / CLAUDE.md lenses

- **consistency** — internal coherence, cross-reference accuracy, contradictions
- **gaps** — missing guidance, undocumented edge cases, assumed context
- **adversarial** — where literal compliance leads to bad agent behavior
- **agent-clarity** — would a cold agent, reading this for the first time, follow it correctly?

### Custom lenses

The lenses above are starting points. You can define arbitrary lenses based on the project's needs — if a project has a specific concern (performance, accessibility, security, concurrency safety, etc.), propose it as a lens. The user can also suggest their own.

**Presets to offer:**
- `comprehensive` — all applicable lenses
- `quick` — 2 most impactful lenses for the project type
- `custom` — user picks

**If $ARGUMENTS is provided**, try to parse it as a preset name or comma-separated lens list and skip the interactive step if unambiguous.

Also ask: which files or directories to focus on? Default: entire codebase. Narrow scope if the user mentions a module or area.

## Step 3: Run parallel audits

Spawn one Agent per agreed lens, all in parallel (single message, multiple Agent tool calls). Each agent is a `general-purpose` agent.

Each agent prompt should:
- State its single lens clearly at the top
- Specify the file scope agreed in step 2
- Instruct the agent to return findings as a structured list: `file:line — issue — suggested fix` (one finding per line)
- Ask for severity: `high` / `medium` / `low`
- Instruct it to be specific and actionable, not vague
- Instruct it NOT to fix anything — observe and report only
- Instruct it to append a **Skipped** section at the end of its report listing any files, modules, or areas it did not fully examine — due to size, complexity, time, or tedium — so the user knows what wasn't covered

## Step 4: Synthesize

Once all agents complete:

1. Deduplicate findings that multiple lenses flagged (keep the most specific description)
2. Flag any conflicts (lens A says expose X, lens B says hide X) — surface these explicitly to the user
3. Group remaining findings by file or by severity
4. Present a clean summary: total findings by lens and severity, then the full list grouped

## Step 5: User review

Present the findings. The user may:
- Approve findings (will be applied next)
- Reject findings (note reason, won't recur)
- Defer findings (keep for later rounds)
- Ask for clarification on any finding

Do not apply any fixes during this step.

## Step 6: Write POLISH.md

Write or update `POLISH.md` in the project root with:

```markdown
# Polish State

Created: <git hash of HEAD when first created, or ISO timestamp>
Last run: <ISO timestamp>
Round: <N>
Project type: <inferred type>

## Lenses
<list of lenses used this session>

## Scope
<files/directories in scope>

## Findings — Round <N>

### <Lens Name>
- [PENDING] `file:line` — issue — suggested fix _(severity: high)_
- [APPROVED] `file:line` — issue — suggested fix _(severity: medium)_
- [REJECTED] `file:line` — issue — suggested fix _(reason: intentional)_

### Conflicts
- <lens A> vs <lens B>: description of conflict — awaiting user decision
```

If POLISH.md already existed, increment the round number and append the new round's findings. Preserve previous rounds.

## Step 7: Offer next steps

Tell the user:
- How many findings are APPROVED and ready to apply
- That they can ask you to apply approved findings now, or run `/polish` again after applying to re-audit
- If there were conflicts, that those need a decision before proceeding

Do not apply fixes unless the user explicitly asks.
