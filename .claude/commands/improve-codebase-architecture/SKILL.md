---
name: improve-codebase-architecture
description: Find deepening opportunities in a codebase, informed by the domain language in CONTEXT.md. Use when the user wants to improve architecture, find refactoring opportunities, consolidate tightly-coupled modules, or make a codebase more testable.
---

# Improve Codebase Architecture

Surface architectural friction and propose **deepening opportunities** — refactors that turn shallow modules into deep ones. The aim is testability and long-term locality.

## Glossary

Use these terms exactly in every suggestion. Consistent language is the point. Full definitions in [LANGUAGE.md](LANGUAGE.md).

- **Module** — anything with an interface and an implementation (function, class, package, slice).
- **Interface** — everything a caller must know to use the module: types, invariants, error modes, ordering, config.
- **Depth** — leverage at the interface: a lot of behaviour behind a small interface.
- **Seam** — where an interface lives; a place behaviour can be altered without editing in place.
- **Adapter** — a concrete thing satisfying an interface at a seam.
- **Leverage** — what callers get from depth.
- **Locality** — what maintainers get from depth: change, bugs, knowledge concentrated in one place.

Key principles:
- **Deletion test**: imagine deleting the module. If complexity vanishes, it was a pass-through. If complexity reappears across N callers, it was earning its keep.
- **The interface is the test surface.**
- **One adapter = hypothetical seam. Two adapters = real seam.**

When the repo has its own domain vocabulary (in CONTEXT.md), that vocabulary takes precedence over LANGUAGE.md terms for domain concepts. Use LANGUAGE.md terms only for architectural structure (module, seam, depth, etc.).

## Process

### 1. Explore

Read `CONTEXT.md` first if it exists. Then use the Agent tool with `subagent_type=Explore` to walk the codebase. Don't follow rigid heuristics — explore organically and note where you experience friction:

- Where does understanding one concept require bouncing between many small modules?
- Where are modules **shallow** — interface nearly as complex as the implementation?
- Where do pure functions get extracted just for testability, but the real bugs hide in how they're called?
- Where do tightly-coupled modules leak across their seams?
- Which parts of the codebase are untested or hard to test through their current interface?

Apply the **deletion test** to anything you suspect is shallow.

### 2. Present candidates

Present a numbered list of deepening opportunities. For each candidate:

- **Files** — which files/modules are involved
- **Problem** — why the current architecture is causing friction
- **Solution** — plain English description of what would change
- **Benefits** — in terms of locality and leverage, and how tests would improve

**Use CONTEXT.md vocabulary for the domain, LANGUAGE.md vocabulary for the architecture.**

Do NOT propose interfaces yet. Ask the user: "Which of these would you like to explore?"

### 3. Grilling loop

Once the user picks a candidate, walk the design tree with them — constraints, dependencies, the shape of the deepened module, what sits behind the seam, what tests survive.

Side effects happen inline as decisions crystallize:

- **Naming a deepened module after a concept not in `CONTEXT.md`?** Add the term — same format as existing entries. Create `CONTEXT.md` now if it doesn't exist yet (don't wait).
- **Sharpening a fuzzy term during the conversation?** Update `CONTEXT.md` right there.
- **Want to explore alternative interfaces for the deepened module?** See [INTERFACE-DESIGN.md](INTERFACE-DESIGN.md).

## CONTEXT.md format

Each entry:

```markdown
## TermName
_Avoid:_ synonym or related term that's easily confused

One-sentence definition that captures what makes this term precise.

What goes wrong when it's confused with the avoided term.
```

Keep entries short. The goal is disambiguation, not documentation.

**Optional sections** (use when they earn their place):

- **Relationships** — when terms have structural connections (cardinality, ownership, lifecycle), add a `## Relationships` section with prose statements like "An Authority owns exactly one Room". Useful when connections between terms are non-obvious.
- **Grouping** — when the glossary grows past ~15 terms and natural clusters emerge (subdomain, lifecycle, actor), group terms under `## Group Name` headings.
