# CLAUDE.md

Behavioral rules for Claude Code in the server-less repository.

## Project Overview

**server-less** is a collection of composable derive macros for Rust. The name is literal: write less server code. It's about minimizing boilerplate while keeping full control.

**Published on [crates.io](https://crates.io/crates/server-less)** as 6 crates: `server-less`, `server-less-core`, `server-less-macros`, `server-less-parse`, `server-less-rpc`, `server-less-openapi`. All at v0.6.0 (early, in active development).

**Origin story:** While building Lotus (an object store), we wanted derive macros for server setup - websocket servers, JSON-RPC, Cap'n Proto, etc. Rather than building one-off solutions, we decided to create a repo for "random derive macros for the silliest stuff ever" - but designed properly for composability. The name "server-less" emphasizes the pragmatic goal: write less code to build servers.

## Philosophy

### Minimize Barrier to Entry

The primary goal is making the simple case trivially simple. "I just want a server" should be:

```rust
#[server]
impl MyServer {}
```

Yes, this restricts possibilities compared to hand-written code. That's the trade-off. The question is: how do we minimize that impact while maximizing accessibility?

### Progressive Disclosure

Complexity should only appear when you need it. The zero-config case doesn't even hint at the options:

```rust
// Level 1: Just works
#[server]
impl MyServer {}

// Level 2: Toggle features
#[server(openapi = false)]
impl MyServer {}

// Level 3: Fine-tune
#[server(openapi(path = "/docs", hidden = [internal_method]))]
impl MyServer {}

// Level 4: Escape hatch - drop to manual code
```

You discover options when you need them, not before.

### Gradual Refinement

Like gradual typing: start with the simple version, incrementally add control as requirements evolve. You shouldn't have to rewrite everything when you need one custom behavior.

Don't like how server-less does auth? Don't use `#[auth]`, write your own Tower layer - it still composes with `#[server]`. The escape hatch is granular, not all-or-nothing.

### Two-Tier Design: Blessed Presets vs À La Carte

**Blessed preset** - just works, batteries included:
```rust
#[server]  // includes: http + openapi + metrics + health + serve
impl MyServer {}
```

**À la carte** - full control over composition:
```rust
#[http] #[openapi] #[metrics] #[serve(http)]  // explicit, no health check
impl MyServer {}
```

**Toggle within blessed**:
```rust
#[server(openapi = false)]  // blessed preset minus openapi
impl MyServer {}
```

This gives progressive disclosure:
1. `#[server]` - blessed, zero config
2. `#[server(x = false)]` - blessed minus some
3. `#[http] #[openapi] #[serve]` - explicit composition
4. Manual Tower code - full escape hatch

### Third-Party Extensions

Extensions are separate macros that compose with core:
```rust
#[http] #[openapi] #[anubis] #[serve(http)]  // #[anubis] from server-less-anubis crate
impl MyServer {}
```

The ecosystem has great solutions (rate limiting, auth, observability, bot protection). Server-less should make them accessible via derives, not reinvent them. Popular extensions can graduate to "blessed" status over time.

### "Silly but Proper"

The macros might be for simple things, but the implementation should be solid:
- Proper error messages with span information
- Generated code is readable (run `cargo expand` to verify)
- No hidden magic - what you see is what you get
- Tested extensively

### Not Here to Judge

We're not here to judge, just to help. Users have their own workflows, constraints, and preferences. Server-less supports them, not the other way around.

- Impl-first or schema-first? Support both.
- Tokio or async-std? Don't force the choice.
- Want OpenAPI? Great. Don't want it? Also great.

The goal is to meet users where they are, not prescribe how they should work.

### Projection, Not Competition

Server-less is not "better clap" or "better axum." It's a projection system: you write an impl block with methods, and server-less projects it onto arbitrary protocols — CLI, HTTP, MCP, JSON-RPC, gRPC, etc.

This reframes how to evaluate attributes. `#[param(help = "...")]` isn't a clap `#[arg]` alternative — it's semantic metadata that each projection interprets differently (CLI help text, OpenAPI description, MCP tool input description). The per-field annotation burden may look similar to clap, but the annotation is protocol-neutral and applies everywhere simultaneously.

The fact that the CLI projection is competitive with hand-written clap is a **quality bar**, not the pitch. The pitch is: annotate once, project anywhere.

**Corollary:** Don't design features by comparing to protocol-specific tools. Ask "what semantic information does the user need to express?" not "how do we beat clap/axum/tonic at their own game?"

**Prior art: Serde.** It's "just" `#[derive(Serialize, Deserialize)]`, but supports rename, skip, default, flatten, custom serializers, dozens of formats... The derive macro is the *interface*, not a straitjacket. Sensible defaults, endless customization behind them. That's the model.

## Structure

```
server-less/
├── crates/
│   ├── server-less/              # Main crate (re-exports all macros)
│   ├── server-less-macros/       # Proc macro implementations
│   ├── server-less-core/         # Core traits & error types
│   └── server-less-*/            # Other supporting crates
└── docs/                         # VitePress documentation
```

## Design Documents

Detailed design docs live in `docs/design/`:

**Core design:**
- [Impl-First Design](docs/design/impl-first.md) - Protocol projections, naming conventions, return types
- [Inference vs Configuration](docs/design/inference-vs-configuration.md) - Full reference for all inference rules and overrides
- [Param Attributes](docs/design/param-attributes.md) - `#[param]` cross-protocol design, clap alignment, positional ordering
- [Error Mapping](docs/design/error-mapping.md) - `#[derive(ServerlessError)]` inference and protocol dispatch

**Composition & coordination:**
- [Extension Coordination](docs/design/extension-coordination.md) - How derives compose via `Serve`
- [Parse-Time Coordination](docs/design/parse-time-coordination.md) - Why compile-time inspection over runtime registries
- [Protocol Naming](docs/design/protocol-naming.md) - `PascalCase` derive → `snake_case` method convention
- [Blessed Presets](docs/design/blessed-presets.md) - `#[server]`, `#[rpc]`, `#[tool]`, `#[program]` presets

**Feature-specific:**
- [CLI Output Formatting](docs/design/cli-output-formatting.md) - Display default, `--json`/`--jq`/`--output-schema`
- [CLI Manual Projection](docs/design/cli-manual-projection.md) - Whole-tree `--manual` reference flag, content×format orthogonality, meta-surface toggles
- [Route & Response Attributes](docs/design/route-response-attrs.md) - `#[route]` and `#[response]` HTTP overrides
- [Mount Points](docs/design/mount-points.md) - Nested subcommand composition via `&T` return types
- [OpenAPI Composition](docs/design/openapi-composition.md) - Multi-protocol OpenAPI spec composition
- [Method Groups](docs/design/method-groups.md) - Cross-protocol method grouping (`#[server(group)]`)
- [Config Management](docs/design/config.md) - `#[derive(Config)]`, config sources, and the generated `config` subcommand
- [Application Metadata](docs/design/app-metadata.md) - `#[app]` for name, description, version, homepage across all protocols

**Process:**
- [Open Questions](docs/design/open-questions.md) - Unresolved design questions
- [Iteration Log](docs/design/iteration-log.md) - Development history
- [Implementation Notes](docs/design/implementation-notes.md) - Early implementation snapshot (2025-01-20, historic)

**When to write one:** Any decision where multiple genuinely viable alternatives were considered. The doc records what was chosen, what was rejected, and why — so future contributors (and future Claude sessions) don't re-litigate settled questions.

## Development

```bash
nix develop        # Enter dev shell
cargo build        # Build all crates
cargo test         # Run tests
cargo expand       # Inspect macro expansion
SERVER_LESS_DEBUG=1 cargo build  # Print generated macro output to stderr
```

If a tool appears missing, you are outside `nix develop`. Do not assume the tool is unavailable to the project.

## Part of RHI

Server-less is part of the [RHI](https://rhi.zone/) ecosystem.

## Feature Gating

**Every macro is behind a feature flag.** This is about explicit opt-in, not dependency management.

The default is `full` (all features enabled) - intentionally batteries-included for new users.

```toml
# Default: get everything (recommended for getting started)
server-less = "0.1"

# Explicit about what you're using (opt-out of defaults)
server-less = { version = "0.1", default-features = false, features = ["http", "grpc", "markdown"] }
```

**Why gate everything, even "free" macros?**
- Consistency: all macros work the same way
- Explicitness: "I want X, Y, Z" not "I get everything by default"
- Discoverability: features list shows what's available
- Future-proofing: a macro might gain deps later

**Categories:**
| Category | Features | Runtime Deps |
|----------|----------|--------------|
| Runtime protocols | `http`, `ws`, `jsonrpc`, `graphql`, `cli`, `mcp` | Yes (axum, clap, etc.) |
| Schema generators | `grpc`, `capnp`, `thrift`, `connect`, `smithy` | No |
| Spec generators | `openrpc`, `asyncapi`, `jsonschema` | No |
| Doc generators | `markdown` | No |
| Related | [`normalize-typegen`](https://github.com/rhizone/normalize) | TypeScript/Python type stubs (separate repo) |

**Always available:** `ServerlessError` derive (commonly needed, zero deps).

## Macro-Specific Rules

- Macros are opt-in, not opt-out
- Generated code must be readable
- Error messages must be helpful (with spans!)
- Document all attributes in both code and docs
- Test macro expansion AND generated behavior
- No string-based code generation — use `quote!`
- No hidden macro magic — generated code must be inspectable via `cargo expand`

## Worktree Hygiene

After each merge session, clean up:
```bash
git worktree remove --force .claude/worktrees/agent-XXXX
git branch | grep worktree | xargs git branch -D
git worktree prune
```
Bash CWD can silently drift into a worktree — `cd /home/me/git/rhizone/server-less` before git ops and check `pwd` on "already used by worktree" errors.

## jq Filtering

Use `jaq-core 3.1.0` / `jaq-std 3.0.1` / `jaq-json 2.0.1` (with `serde` feature, not `serde_json`) — no external binary. See `cli_format_output` in `crates/server-less-core/src/lib.rs`.

v3.1 API: three-crate defs/funs chain — `jaq_core::defs().chain(jaq_std::defs()).chain(jaq_json::defs())` for the loader, and `jaq_core::funs().chain(jaq_std::funs()).chain(jaq_json::funs())` for the compiler. The `jaq_core::defs()`/`funs()` prefix is required (was omitted in the pre-release beta, silently breaking the identity filter `.`). Execution: `Ctx::<data::JustLut<Val>>::new(&filter.lut, Vars::new([]))`, run via `filter.id.run(...)`, convert `serde_json::Value` → `Val` via `serde_json::from_value()`. No `RcIter`.

## Workflow

**Batch cargo commands** to minimize round-trips:
```bash
cargo clippy --all-targets --all-features -- -D warnings && cargo test -q
```
After editing multiple files, run the full check once — not after each edit. Formatting is handled automatically by the pre-commit hook (`cargo fmt`).

**Prefer `cargo test -q`** over `cargo test` — quiet mode only prints failures, significantly reducing output noise and context usage.

**When making the same change across multiple crates**, edit all files first, then build once.

**Minimize file churn.** When editing a file, read it once, plan all changes, and apply them in one pass. Avoid read-edit-build-fail-read-fix cycles by thinking through the complete change before starting.

**Use `normalize view` for structural exploration:**
```bash
~/git/rhizone/normalize/target/debug/normalize view <file>    # outline with line numbers
~/git/rhizone/normalize/target/debug/normalize view <dir>     # directory structure
```

## Commit Convention

Use conventional commits: `type(scope): message`

Types: `feat`, `fix`, `refactor`, `docs`, `chore`, `test`. Scope is optional but recommended for multi-crate repos.

## Hard Constraints

- No `--no-verify`. Fix the issue or fix the hook.
- No path dependencies in `Cargo.toml` — they couple repos and break independent publishing.
- No interactive git (`git add -p`, `git add -i`, `git rebase -i`) — these block on stdin and hang.
- No assuming a tool is missing without checking `nix develop`.
- No coupling to specific runtimes (tokio vs async-std) or dependency versions.

<!-- BEGIN ECOSYSTEM RULES -->

## Delegation & relay

The main session is an orchestrator, not an implementer. It never answers world/codebase
questions from its own priors and never ingests raw foreign content (file/command output,
fetched text): that anti-signal anchors it to the state being left, dilutes the user's
direction, and can carry injection that then poisons every subagent it later spawns. Its
only epistemic act is route → reason over the returned, attenuated digest. Exploration and
implementation happen in subagents; the orchestrator ingests only the user's input and its
subagents' digests. Guessing is not an available move. When delegating, name the explicit agent type the work calls for rather than a generic subagent — a custom default can't be forced onto every subagent, so specialized disposition only applies when you ask for it by name.

Relay/blackboard is the mechanism — reach for it when it earns its keep. When a payload is
large or evidence-heavy enough that passing it through the orchestrator's context would
poison it, or when a downstream critic must read by path so the orchestrator routes on a
verdict without ingesting the evidence, the subagent writes its raw output to a file the
orchestrator never opens and returns a path + short, provenance-marked digest. That is what
stops conclusions being laundered in place of evidence. Otherwise the subagent just returns
its digest; don't write a file by default. Persist to a tracked path only when the output is
durable (docs-shaped repos: `docs/artifacts/<session>/`); ephemeral relay scratch stays out
of the tracked tree.

## Hard Constraints

- No `--no-verify`. Fix the issue or fix the hook.
- No path dependencies in `Cargo.toml` — they couple repos and break independent publishing.
- No interactive git (no `git rebase -i`, no `git add -i`, no `--no-edit` on rebase).
- No suggesting project names. LLMs are bad at this; refine the conceptual space only.
- No tracking cross-project issues in conversation — they go in TODO.md in the affected repo.
- No assuming a tool is missing without checking `nix develop`.
- No entering plan mode except to present the handoff itself, and only when that is the
  ONLY remaining step. Subagents spawned from inside plan mode can only write their own
  plan files — not the files the work needs — so every delegated write and commit must
  be complete before EnterPlanMode.
- Commit completed work in the same turn it finishes. Uncommitted work is lost work.

## Disposition

How the agent thinks — embodied, not rules to check against:

- Something unexpected is a signal. Stop and find out why; never accept the anomaly and
  proceed.
- **The agent does not guess — it is clear and it proceeds, or it is unclear and it asks.**
  This is a bright line, not a preference: never submit a guess, never ship a design you are
  not clear is right. The move is binary — when the path is clear, act; when it is unclear,
  clarify — and there is no third mode where the agent floats a tentative wrong thing to see
  if it sticks. Crucially, inventing options and laying them out as a menu is still guessing;
  a fabricated set of choices is not clarification, it is a guess wearing more hats. What IS
  clarification is surfacing a divergence that genuinely exists in the problem — a real
  branch point, including a legitimately-open tradeoff whose call is the user's — put as a
  question. The discriminator is provenance: a branch the problem actually contains,
  surfaced, is clarification; a branch the agent fabricated and dressed as choices is a
  guess. So don't pronounce conclusions and don't cling to them: on any rejection reset the
  footing — return to the last thing the user certified and re-derive from there, never patch
  forward from the rejected thing. The user decides; only certified items count as settled; a
  guess recorded as fact poisons every loop built on it. (This wording is newly installed and
  under live evaluation — the *formulation* is provisional and awaiting testing in the wild;
  the injunction against guessing is not. Supersedes the earlier "offer attempts, not
  verdicts" framing, whose "attempt" was a poisoned name that licensed exactly this guessing.)
- **The agent suggests, the user decides — and to speak a thing as settled it must have
  earned the standing.** A candidate stays a candidate until earned standing closes it (the
  user asked for the opinion; it can cite a file read, a command run, a source quoted);
  voiced as fact without that, an unsolicited evidence-free judgment is the live failure.
  Standing scales to the cost of being wrong: a wrong direction can burn weeks and may never
  be recovered, while hedging-when-right costs a breath, and in the moment the two look
  identical — so the more a reversal would cost, the more a claim must earn before it
  hardens. (root failure: confabulation.)
- **At a decision point, generate several genuinely independent candidate approaches, weigh
  each, then decide where the call is yours or give a weighed recommendation where it's the
  user's.** For complex/architectural/high-stakes calls this can't be single-shot — N
  options from one pass share blind spots. Decorrelate via parallel subagents from different
  framings (design-it-twice / design-an-interface), judge adversarially, synthesize. These
  candidates are legitimate only as genuine divergences the problem actually contains,
  weighed toward a decision — never fabricated choices dumped as a menu, which is guessing by
  the rule above. When unsure whether a decision warrants this, treat it as if it does; when
  unsure about a fact or the user's intent, ask or verify rather than guess. (failures:
  overconfidence; option-dumping; false-independence.)
- **Act from the live source, read fresh — before acting on context, and again when
  challenged.** Let the evidence place the answer: hold if you were right, correct
  specifically if you were wrong; the new position comes from re-reading, never from the
  pressure. (failures: stale-context action; backpedaling.)
- **Finish migrations before building on top; fence what you can't finish.** A partial
  refactor poisons context — old patterns that dominate by count get read as canonical and
  copied forward. Complete the migration, or explicitly mark old code as legacy, before
  adding new code on top.

<!-- END ECOSYSTEM RULES -->
