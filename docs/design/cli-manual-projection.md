# CLI Manual Projection

A whole-tree reference projection for `#[cli]`: one invocation that emits the
entire command surface as a single document — the tool's "manual".

## Decision

Add a global flag to the `#[cli]` projection that emits the **entire command
tree** as one reference document, rather than a single leaf's surface. The flag
selects *content* (the whole tree); the existing global format flags
(`--json` / `--jsonl` / `--jq`) select *shape*. The aggregate contains each
node's input schema, output schema, and description as elements. The flag —
along with the existing `--input-schema` / `--output-schema` meta-surfaces — is
toggleable globally and optionally per-command.

The flag's **name** (`--manual` vs `--manpage`), its **default output format**,
and the **exact toggle syntax** are left open below.

## Context

Every meta-surface in the `#[cli]` projection today is strictly **per-leaf**.
`--input-schema` and `--output-schema` are global clap flags, but each leaf only
ever emits *its own* schema — they are generated exclusively inside
`generate_leaf_match_arm` and are never attached to mount-point (container)
arms (see [schema-flag-wiring.md](../artifacts/normalize-cli-docs/schema-flag-wiring.md),
"What is built"). There is no whole-tree aggregation: no single invocation that
dumps the entire command surface as one document.

This bites real tools. normalize exposes **~150 named commands at depth 3** via
nested `#[cli]` services
(see [cli-surface.md](../artifacts/normalize-cli-docs/cli-surface.md) §2). A
deeply-nested CLI of that size is not greppable from the shell: there is "no
`normalize help --all` or `normalize --help-all` that prints the entire tree in
one call" and "no single `--output-schema` at root level that dumps all command
schemas" (cli-surface.md §3, "What does NOT exist").

The `#[markdown]` / `#[program]` preset *does* generate a `markdown_docs()`
method — but it is a **Rust method, not a CLI surface**. It is Rust-callable
only and is not wired to any flag or subcommand. normalize uses bare `#[cli]`
(not `#[program]`), so `markdown_docs()` is not even available on its service
(cli-surface.md §3). A user at the shell therefore cannot grep the docs of the
tool they are holding. This gap was hit live while working on normalize.

So the user-facing need is concrete: **"let me grep the whole tool's surface
from the shell, in a shape I can pipe."** The default-`Display` /
`--json` / `--jq` content×format convention already established in
[CLI Output Formatting](cli-output-formatting.md) gives us the shape axis for
free. What is missing is a *content* selector for "the whole tree."

## Design

### A content flag, orthogonal to format

The new flag selects **content** — the whole command tree — and composes with
the **format** flags server-less already ships
([CLI Output Formatting](cli-output-formatting.md)):

| Invocation | Result |
|---|---|
| `tool --manual` | Whole tree, human-readable rendering |
| `tool --manual --json` | Whole tree as one structured doc keyed by command path; jaq-queryable |
| `tool --manual --jsonl` | Whole tree, one JSON line per node |
| `tool --manual --jq '...'` | Whole tree, inline jaq query applied |

This is the same content×format split the projection already uses everywhere
else: a content selector says *what* to emit, the format flags say *what shape*.
The manual flag invents no new format concept — it reuses `--json` / `--jsonl` /
`--jq` verbatim. (`--manual` is shown as the working name; see
[Open Questions](#open-questions).)

### The aggregate is keyed by command path

Under `--json`, the document is keyed by command path, each node carrying its
input schema, output schema, and description:

```jsonc
{
  "view list":        { "description": "...", "input_schema": { ... }, "output_schema": { ... } },
  "view chunk":       { "description": "...", "input_schema": { ... }, "output_schema": { ... } },
  "edit history list":{ "description": "...", "input_schema": { ... }, "output_schema": { ... } },
  // ... every leaf in the tree
}
```

This shape is exactly what makes the surface greppable:

```bash
tool --manual --jq 'keys[] | select(startswith("edit"))'    # every command under `edit`
tool --manual --jq '."view list".input_schema'              # one leaf's input schema
tool --manual --jq 'to_entries[] | select(.value.description | test("rename"))'
```

Each node's schema appears in the aggregate as **one element**. This is the
crux of why position-overloading was rejected (below): a leaf's own schema is a
*member* of the manual, never in competition with it. `tool view list
--output-schema` still means "that leaf's output schema"; `tool --manual` means
"every leaf's surface, the manual containing them." The two never overlap.

### Default format (open, with a lean)

What `tool --manual` produces with no format flag is left **open** (see
[Open Questions](#open-questions)). The lean is **human-readable / markdown**:
a bare `--manual` should be readable prose (a rendered reference page), not a
wall of JSON — consistent with the projection defaulting to `Display` rather
than JSON ([CLI Output Formatting](cli-output-formatting.md)). Structure is then
one flag away (`--manual --json`). This keeps the zero-argument case friendly
while leaving the machine path trivially reachable.

### --manual works at every node, not just the root

`--manual` is valid at **every position in the command tree** — root, internal
mount point, and leaf alike:

```bash
normalize --manual                   # whole tree
normalize edit --manual              # the `edit` subtree only
normalize edit history list --manual # that single leaf's entry
```

This is not an overload. `--manual` has **one consistent meaning** at every
position: "render the reference document for the subtree rooted here." The root
is simply the largest subtree; a leaf is a subtree of one. What changes by
position is only the **extent** — it is a scope parameter, not a
position-dependent meaning.

This must not be read as reversing the rejection of position-overloading
`--output-schema` (see Alternatives Considered). That rejection was about a flag
with *two different operations* by position: "this command's return-type schema"
at a leaf versus "aggregate of everything" at the root, which collapsed to
ambiguous at a `#[cli(default)]` root where both readings were simultaneously
valid. `--manual` has *one* operation ("manual of the subtree here") everywhere;
no node has two valid readings. The distinction is:

- **Scope parameter:** same operation, varying extent — valid everywhere, no
  ambiguity. This is `--manual`.
- **Meaning overload:** different operations by position — ambiguous at nodes
  where both readings apply. This is what was rejected for `--output-schema`.

The natural mental model: clap's `--help` already works at every node. `--manual`
is the whole-subtree version of the same idea, emitted as one document rather
than one leaf's usage. "Works on subcommands" is therefore the expected behavior,
not a special case.

Per-subtree scoping is also **genuinely useful** for large trees. normalize
exposes ~150 commands: `normalize edit --manual` narrows the document to the
`edit` group without any additional filtering — built-in grep-narrowing at the
CLI level, before `--jq` is even involved.

**The mechanism gives it for free.** `--manual` is a global flag (like
`--output-schema` / `--input-schema`), so clap already propagates it to every
subcommand parser in the tree. Making `--manual` work at every node is the
*default* of that propagation; restricting it to root-only would require
explicit extra suppression and would be inconsistent with how the other global
meta-surface flags behave. There is no extra implementation cost to the
everywhere behavior — the cost flows the other way.

### Toggling the meta-surfaces

The manual flag and the existing `--input-schema` / `--output-schema` flags are
**meta-surfaces** the projection injects automatically. They must be
**disable-able** — globally, and optionally per-command. Today these flags are
hardcoded always-on in the `format_flags` block with **no toggle and no
reserved-name collision guard**
(see [schema-flag-wiring.md](../artifacts/normalize-cli-docs/schema-flag-wiring.md)).
A tool that has a legitimate `--manual` method parameter, or that does not want
to expose schemas at all, currently has no recourse.

The toggle model should align with the projection's existing
progressive-disclosure idiom — the same `x = false` shape as
`#[server(openapi = false)]`
(see [Blessed Presets](blessed-presets.md), "Toggles Bridge the Tiers"). The
**exact syntax is open** (see [Open Questions](#open-questions)); a shape
consistent with that idiom, *for illustration only*:

```rust
// Global: drop the manual surface entirely
#[cli(name = "tool", manual = false)]
impl MyService { ... }

// Global: drop the per-leaf schema flags too
#[cli(name = "tool", input_schema = false, output_schema = false)]
impl MyService { ... }

// Per-command: hide one leaf from the aggregated manual
impl MyService {
    #[cli(manual = false)]
    pub fn internal_method(&self) -> Report { ... }
}
```

The principle, not the spelling, is what this doc fixes: meta-surfaces are
**configurable**, default-on, and follow the `= false` toggle convention. The
spelling is deferred to implementation.

### Implementation: aggregation mechanism (decided)

The load-bearing new piece is **whole-subtree aggregation** — walking the command
tree and collecting each node's schema + description. Two viable mechanisms were
weighed:

1. **Compile-time codegen that materializes the tree** (chosen). The `#[cli]`
   macro adds a `cli_manual_nodes(&self, prefix) -> Vec<CliManualNode>` method to
   the `CliSubcommand` trait. Each impl emits one node per leaf (reusing the exact
   input/output-schema codegen the `--input-schema`/`--output-schema` flags
   already use) and, for each mount point, recurses by calling the child type's
   `cli_manual_nodes` with an extended path prefix. Aggregation composes through
   the **same mount recursion the dispatcher already uses**, so nesting and `&T`
   composition come for free.

2. **Runtime walk of clap's `Command` tree + per-leaf schema builders.** Rejected:
   clap's `Command` carries no return-type information, so the output schema would
   have to be re-threaded by command name anyway — duplicating the schema logic and
   re-deriving by string lookup what the macro already knows by type at the leaf.
   It also scrapes runtime state rather than emitting a faithful data structure.

**Why (1):** it keeps the manual a *serializable data structure*
(`Vec<CliManualNode>`, rendered to path-keyed JSON or text by
`cli_manual_to_json` / `cli_manual_to_text` in `server-less-core`), not scraped
runtime state — consistent with the ecosystem's "prefer data over code at a seam"
principle. It reuses the existing leaf schema codegen verbatim, and the recursion
mirrors dispatch, so a mount subtree is aggregated exactly where it is dispatched.

**Resolved sub-cases:**

- **Mount points (`&T`):** static mounts (`fn(&self) -> &T`) recurse into the
  child's `cli_manual_nodes` with the mount name appended to the prefix — the full
  subtree appears inline, depth-unbounded.
- **Slug mounts (`fn(&self, id) -> &T`):** the child is parameterized by a runtime
  slug value the manual cannot synthesize, so an instance cannot be constructed at
  manual time. These contribute a **single container node** (path includes the
  `<slug>` placeholder, description notes "invoke with a slug value and `--manual`
  for its subtree"). The per-leaf detail remains reachable via
  `tool <slug-mount> <id> --manual`, which scopes to that subtree at runtime.
- **`#[cli(default)]`:** the default leaf is an ordinary leaf and so appears as one
  node in the aggregate. The `--manual` interception runs at the **top of
  `cli_dispatch`, before** the default-action `None` arm, so `tool --manual` means
  "the manual of the whole tree", not "run the default action" — the default leaf's
  own entry is simply one node within that manual. This is the ambiguity
  schema-flag-wiring.md flagged, dissolved exactly as the alternative analysis
  predicted.
- **Subtree scoping:** `--manual` is intercepted at every node. When set with no
  further subcommand selected (`matches.subcommand().is_none()`), the current node
  emits `cli_manual_nodes("")` for its subtree. A leaf the user navigated to emits
  its own single-node entry in its match arm. Mount arms fall through and recurse,
  so the child node performs the emission — giving `tool foo --manual` the `foo`
  subtree for free from clap's global-flag propagation.

**Resolved from Open Questions by this implementation:**

- **(a) Flag name:** `--manual` (matches the prompt's fixed decision and reads
  naturally at depth).
- **(b) Default format:** human-readable text (markdown-ish), via
  `cli_manual_to_text`; `--json`/`--jsonl`/`--jq` select the structured shape via
  `cli_manual_to_json` + the existing `cli_format_output`.

Still **out of scope** (deliberate follow-up, unchanged): (c) the meta-surface
disable toggles and (d) the reserved-name collision guard. With no collision guard
yet, a user parameter literally named `manual` would collide with the injected
global flag — noted, not fixed here.

## Alternatives Considered

### A blessed `docs` / `man` subcommand auto-injected into every CLI

Inject a `tool docs` (or `tool man`) subcommand into every `#[cli]` that prints
the whole-tree reference.

**Rejected: namespace-stomp.** A reserved subcommand permanently claims a name
in *every* downstream CLI's command namespace. It collides with any user method
that happens to be named `docs` / `man`, and — once shipped — it cannot be
removed without a breaking change to every tool that depends on it. The
projection's job is to surface the *user's* methods as commands; silently
seizing a name for our own purposes inverts that. A global flag carries the same
capability without consuming a command name. (The collision risk is not
hypothetical: schema-flag-wiring.md notes there is currently **no
reserved-name collision guard** at all — adding a *subcommand* would make the
gap worse, not better.)

### Position-overloading the existing `--output-schema`

Make `--output-schema`'s meaning depend on its position in the tree: at the
**root** it emits the aggregate whole-tree schema; at a **leaf** it emits that
command's return-type schema.

**Rejected on two grounds.**

1. **It poisons the mental model regardless of collisions.** A flag whose
   meaning depends on *where* it appears is a flag the user can never reason
   about locally: "`--output-schema` means this command's schema, except at the
   root where it means every command's schema." Even with no colliding node in
   the tree, the rule is "X here, Y there" — exactly the kind of positional
   special-case the projection avoids. The clean rule —
   **`--output-schema` means *this command* at any position; the manual is a
   separate flag** — falls directly out of rejecting the overload.

2. **It collides concretely with `#[cli(default)]`.** When a default leaf is
   set, `tool --output-schema` (no subcommand) *already* means "the default
   leaf's output schema." schema-flag-wiring.md identifies this as "the one
   ambiguity to resolve": overloading would force `--output-schema` at the root
   to mean *either* the default leaf's schema (current behavior) *or* the
   aggregate — and it cannot mean both. A separate manual flag dissolves the
   ambiguity: the default leaf keeps `--output-schema`, the manual gets its own
   flag, and the aggregate simply *contains* the default leaf's schema as one
   node.

Rejecting the overload is also what makes the **toggle requirement** clean: with
`--output-schema` meaning one thing everywhere and the manual being a distinct
flag, each meta-surface has a single, independently-toggleable identity. There
is nothing positional to special-case in the disable path.

## Open Questions

These are deliberately left unresolved — recorded so they are not silently
decided by implementation:

- **(a) Flag name: `--manual` vs `--manpage` vs `--help-manual`.** `--manual`
  is broader and makes no promise about format or length. `--manpage` evokes
  `man(1)` — familiar and evocative — but connotes a *single page*, which sits
  awkwardly against a ~150-command tree and against the `--json` aggregate shape.
  A third candidate, `--help-manual`, sorts adjacent to `--help` in `--help`
  output (discoverability) and signals the connection to the help system; but
  `--manual` reads more naturally at depth (`normalize edit --manual` vs
  `normalize edit --help-manual`). The discoverability vs. call-site readability
  tradeoff is unresolved. No pick made.

- **(b) Default output format** of a bare `--manual` (no format flag). Lean:
  human-readable / markdown, with structure one flag away (`--manual --json`).
  Recorded as a lean, not a decision.

- **(c) Exact toggle / disable syntax** for the meta-surfaces. The doc fixes the
  *principle* (default-on, `= false`, global and optionally per-command,
  matching `#[server(openapi = false)]`); the precise attribute spelling is
  deferred.

- **(d) The pre-existing no-collision-guard gap.** There is currently no
  reserved-name guard for any injected flag
  (schema-flag-wiring.md). Each new global flag the projection adds — `--manual`
  included — makes this more load-bearing: a user method or parameter named
  `manual` would silently collide. Reconciling the manual flag with a general
  reserved-name strategy is left open and grows in importance as the global-flag
  set grows.

## See Also

- [CLI Output Formatting](cli-output-formatting.md) — the content×format
  convention this reuses; `--json` / `--jsonl` / `--jq`, `--input-schema` /
  `--output-schema`
- [Blessed Presets](blessed-presets.md) — the `#[server](x = false)` toggle
  idiom the disable model follows
- [Mount Points](mount-points.md) — the leaf-vs-container partition the aggregate
  walks
- [cli-surface.md](../artifacts/normalize-cli-docs/cli-surface.md) — normalize's
  ~150-command tree and the "what does NOT exist" gap (no whole-tree dump,
  `markdown_docs()` is Rust-only)
- [schema-flag-wiring.md](../artifacts/normalize-cli-docs/schema-flag-wiring.md)
  — meta-surfaces are leaf-only, always-on, with no toggle and no collision
  guard; the `--output-schema` / `#[cli(default)]` ambiguity
