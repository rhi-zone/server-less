---
name: design-it-twice
description: Use any time anything needs to be designed — any design or architecture decision, code or not: APIs, data models, algorithms, module boundaries, schemas, protocols, naming/conceptual structure, system shape. Generates genuinely independent candidate designs via parallel sub-agents given decorrelated framings, judges them adversarially, and synthesizes a winner. Surfaces the design-it-twice discipline whenever a decision is being made instead of leaving it buried.
---

# Design It Twice

Based on "Design It Twice" (Ousterhout): your first idea is unlikely to be the best. But the real failure mode is subtler than "didn't try enough options" — it's that options from a single pass aren't independent. This skill makes them independent, then makes them fight.

## 1. Gate on complexity and stakes

Not every decision earns the full treatment. A trivial, local, easily-reversible choice gets one pass: weigh it, decide, move on. The full procedure is for decisions that are complex, architectural, high-stakes, or hard to reverse — the ones where a confidently-wrong design is expensive to undo later.

**When you're unsure whether a decision clears the bar, treat it as if it does.** Over-applying wastes some compute. Under-applying ships a confident wrong design that someone pays for downstream. The asymmetry says: when in doubt, design it twice.

## 2. Why single-shot fails

Asking one model pass for N candidates does not give you N independent designs. They share a framing, a set of unstated assumptions, and the same blind spots — they're one design reworded N times. If there's a root error in the shared framing, *every* candidate inherits it, and comparing them can't surface it because they all agree on exactly the wrong thing.

So the goal is not "more options." It's **decorrelated** options — candidates whose errors are independent, so that where one is blind another can see.

## 3. Generate decorrelated candidates (parallel sub-agents)

Spawn sub-agents in parallel (Agent tool, one message). Each gets the **same brief** — the problem, constraints, vocabulary, what's behind the seam — but a **different starting frame** that forces it down a different part of the space. They must not see each other's output; independence is the whole point.

The frames below are illustrative. Pick frames that actually fan out *this* decision — the right set depends on the problem:

- **Minimize / subtract** — fewest moving parts, fewest concepts; find the one primitive that makes the special cases stop being special.
- **Maximize flexibility** — support the widest range of futures and extension.
- **Optimize the common case** — make the dominant caller's path trivial, accept cost elsewhere.
- **Invert the dependency** — flip who owns/calls whom; push the decision to the other side of the seam.
- **Different conceptual primitive** — rebuild on a different core noun (a stream instead of a table, an AST instead of a string, a capability instead of a name).

Each candidate states: the design; a concrete usage/realization example; what it hides or assumes; and its own honest trade-offs — where it's strong, where it's thin.

## 4. Judge adversarially

Do not let candidates self-grade, and do not eyeball them yourself with a single confident pass — that reintroduces the correlated-blindness this skill exists to defeat. Spawn independent judges / skeptics whose job is to **attack** each candidate: find the input that breaks it, the assumption that won't hold, the cost it hid, the case it can't represent. A candidate's score is how well it survives the attack, not how good it sounded.

## 5. Synthesize and recommend

Take the survivor as the base and graft the best ideas from the runners-up — the strongest design is often a hybrid, and the adversarial round tells you which grafts are safe. Where the decision is yours to make, make it and say why. Where it's the user's, present the weighed comparison with a clear recommendation and the reasoning behind it — not a bare menu to re-analyze.

If the candidates disagreed at the root — not on details but on the framing itself — that disagreement is the signal. It means the shared framing a single pass would have used was wrong, and the gap between candidates is exactly where the bug was hiding. Resolve it explicitly rather than papering over it.

## See also

- **design-an-interface** — the module-interface-specific specialization of this procedure. For designing an API, a module's public surface, or a seam shape, use it directly; it carries the architectural vocabulary (depth, leverage, locality, adapter) that interface design needs.
- **think-with-the-engineering-taste** — the lens, not the loop. Run it first when you want the candidate-generating and judging to reach for the right moves (subtract, prefer data, project from one definition) by reflex.
