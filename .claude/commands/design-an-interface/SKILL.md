---
name: design-an-interface
description: Generate multiple radically different interface designs for a module using parallel sub-agents. Use when user wants to design an API, explore interface options, compare module shapes, or mentions "design it twice".
---

# Design an Interface

Based on "Design It Twice" (Ousterhout): your first idea is unlikely to be the best. Generate multiple radically different designs in parallel, then compare.

Uses the architectural vocabulary from [improve-codebase-architecture/LANGUAGE.md](../improve-codebase-architecture/LANGUAGE.md) — **module**, **interface**, **seam**, **adapter**, **depth**, **leverage**, **locality**.

## Workflow

### 1. Gather Requirements

Before designing, understand:

- What problem does this module solve?
- Who are the callers? (other modules, external users, tests)
- What are the key operations?
- Constraints? (performance, compatibility, existing patterns)
- Dependency category — in-process, local-substitutable, remote-but-owned, true-external (see [improve-codebase-architecture/DEEPENING.md](../improve-codebase-architecture/DEEPENING.md))
- If a `CONTEXT.md` exists in the repo, read it — domain vocabulary belongs in the design

Ask the user the answers you can't get from the codebase. Don't proceed until you have enough to write a meaningful brief.

### 2. Generate Designs (Parallel Sub-Agents)

Spawn 3 sub-agents in parallel using the Agent tool. Each gets the same brief plus a different design constraint:

- Agent 1: "Minimize the interface — aim for 1–3 entry points max. Maximize leverage per entry point."
- Agent 2: "Maximize flexibility — support many use cases and extension."
- Agent 3: "Optimize for the most common caller — make the default case trivial."

(Add a fourth if cross-seam dependencies are involved: "Design around ports & adapters for cross-seam dependencies.")

The brief includes file paths, dependency category, what sits behind the seam, and both LANGUAGE.md and CONTEXT.md vocabulary so designs use consistent naming.

Each sub-agent outputs:

1. **Interface** — types, methods, params, plus invariants, ordering, error modes
2. **Usage example** — how callers actually use it
3. **What the implementation hides behind the seam**
4. **Dependency strategy and adapters**
5. **Trade-offs** — where leverage is high, where it's thin

### 3. Present and Compare

Show each design sequentially so the user can absorb each one. Then compare in prose (not tables) by **depth** (leverage at the interface), **locality** (where change concentrates), and **seam placement**. Highlight where designs diverge most.

### 4. Recommend

Give your own opinionated read: which design is strongest and why. If elements from different designs combine well, propose a hybrid. The user wants a strong recommendation, not a menu.

## Anti-Patterns

- Don't let sub-agents see each other's work — independence is the point
- Don't skip comparison — the value is in the contrast
- Don't implement — this is about interface shape, not internals
- Don't evaluate based on implementation effort — evaluate on the interface
