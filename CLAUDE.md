# CLAUDE.md

Behavioral rules for Claude Code in the server-less repository.

## Project Overview

**server-less** is a collection of composable derive macros for Rust. The name is literal: write less server code. It's about minimizing boilerplate while keeping full control.

**Published on [crates.io](https://crates.io/crates/server-less)** as 6 crates: `server-less`, `server-less-core`, `server-less-macros`, `server-less-parse`, `server-less-rpc`, `server-less-openapi`. All at v0.2.0 (early, in active development).

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

## Context Is The Only Scarce Resource

Every byte that enters the main session stays in the main session for its entire lifetime. File contents, command output, search results — once read, it lingers in cache and shapes every downstream token. There is no "just looking."

**All exploration runs in subagents.** Investigations, audits, deep dives, surveys, "let me check," "let me find" — if the purpose of a tool sequence is to find out something you don't yet know, it runs in a subagent. Renaming the activity does not change what it is. The subagent returns a distilled summary; the raw output stays in the subagent.

The main session holds only the durable artifacts you are producing: the edit, the commit, the doc update.

Inline tool use in the main context is reserved for:
- Reading a known file at a known path
- Edits/writes you're committing to
- A single targeted lookup whose result you'll act on immediately

If you find yourself running a second grep to refine the first, you should have spawned a subagent.

- Codebase-wide analysis (architecture, patterns, cross-file survey) → always subagent
- Mechanical work across many files (applying the same change everywhere) → parallel subagents
- **Code changes beyond a trivial single-file edit → worktree agent.** Read the files, write the prompt with exact file paths and what to change, then delegate. Do not implement non-trivial changes inline.

## Subagent Prompts

A subagent prompt is composed in a "spec-writing" register that subtly changes what feels in-scope. Specific failure modes to name:

**Never tell a subagent "do not commit."** Delegation does not strip the commit step from completed work. If a subagent modifies files and the work is done, either the subagent commits, or the next thing the delegator does after it returns is commit — not summarize, not report. The phrase "do not commit" in your own prompt is the tell that you are about to leave work uncommitted.

**Do not delegate judgment.** Phrases like "if extraction is awkward, just duplicate" or "based on your findings, fix the bug" push synthesis onto the agent. If you are punting a decision into the prompt, you do not yet have enough understanding to delegate. Investigate first; write the prompt with the decision already made.

**Do not ask for a diff summary.** Subagent self-reports describe intent, not effect. After a code-modifying subagent returns, read `git diff` yourself. Skip the "report what you changed" instruction — it produces text you cannot trust and that pollutes main context.

**Do not re-explain CLAUDE.md.** Subagents inherit it. Repeating project layout or repo conventions in the prompt dilutes the actual task instructions and signals half-trust in the inheritance. Trust it or don't read it.

**Line numbers are orientation, not anchors.** Files shift between your read and the subagent's read. When citing locations, tell the subagent to find the lines by content ("the block that does X"), not by number.

**Name files explicitly; do not outsource the grep.** "Wherever it appears" invites scope creep. Grep first, list the exact files in the prompt.

**If the task is smaller than the prompt describing it, do it inline.** A subagent dispatch pays a full system-prompt + CLAUDE.md cache cost. One-shot bash commands and single-line edits should run in the main session with `Bash` or `Edit`.

**Match agent type to deliverable shape.** `Explore` is for lookup and search — finding files, symbols, references — not analytical synthesis. For audits, surveys, and pattern analysis whose deliverable is a report, use `general-purpose` with an explicit Opus model. For tasks whose deliverable is files on disk, use `general-purpose` with the tier matched to the work (Sonnet for mechanical, Opus for architectural).

**On unsatisfying subagent output, change something before retrying.** Same prompt + same model + same agent type = same result. Escalate model tier (Sonnet → Opus), narrow the prompt, or switch agent type. Identical retries are waste.

**Dispatch independent subagents in parallel.** Multiple Agent tool_use blocks in a single assistant message run concurrently. Serial Agent dispatch across sequential turns is the default failure mode and trades wall time for nothing. If two subagents do not depend on each other's output, they belong in the same message.

**Pair `isolation: worktree` with `run_in_background: true`.** A worktree implies meaningful write work. Foregrounding it blocks the main session for the entire run. Background unless the worktree's immediate output is what you need to act on next.

**Always set `subagent_type` and `model` explicitly.** Defaulting either collapses tier choice into an invisible decision. The model and agent type are part of the spec; name them every time, even when the choice is obvious. See the existing `Subagent model tiers` section above for which tier fits which work.

## Durability

Subagent reports, mid-session realizations, "I'll remember this" — none of these outlast the session. Anything worth keeping goes into CLAUDE.md, code, docs, or a commit. If it isn't written down, it is gone.

**Commit completed work immediately.** After tests pass, commit. After each phase of a multi-phase plan, commit. Uncommitted work is lost work, and accumulated uncommitted phases lose isolation as well.

**Docs change in the same commit as the code.** New pages enter the sidebar in that commit. There is no follow-up.

Problems, tech debt, issues → TODO.md now, in the same response. Design decisions, key insights → docs/ or CLAUDE.md. Future/deferred scope → TODO.md **before** writing any code, not after. If you write a TODO comment in source, the next action is to open TODO.md and write the entry.

## Authenticity

When asked to analyze X, read X. Do not synthesize from conversation memory, prior summaries, or what the file probably says. Claims must correspond to evidence produced this session.

**Something unexpected is a signal.** Surprising output, anomalous numbers, a file containing what it shouldn't — stop and find out why. Do not accept the anomaly and proceed.

**Corrections are documentation lag, not model failure.** When the same mistake recurs, the fix is writing the invariant down — not repeating the correction. Every correction that doesn't produce a CLAUDE.md edit will happen again. Exception: during active design, corrections are the work itself — don't prematurely document a design that hasn't settled yet.

## Discipline

Corrections from the user are conversation, not material for new rules. A single correction does not warrant a CLAUDE.md edit. Rules are added when a failure mode is observed repeatedly and the rule names the failure it prevents.

Do not announce actions ("I will now…"). Act.

**Macro-specific rules:**
- Macros are opt-in, not opt-out
- Generated code must be readable
- Error messages must be helpful (with spans!)
- Document all attributes in both code and docs
- Test macro expansion AND generated behavior

## Worktree Hygiene

After each merge session, clean up:
```bash
git worktree remove --force .claude/worktrees/agent-XXXX
git branch | grep worktree | xargs git branch -D
git worktree prune
```
Bash CWD can silently drift into a worktree — `cd /home/me/git/rhizone/server-less` before git ops and check `pwd` on "already used by worktree" errors.

## jq Filtering

Use `jaq-core 3.0.0-beta` / `jaq-std 3.0.0-beta` / `jaq-json 2.0.0-beta` (with `serde` feature, not `serde_json`) — no external binary. See `cli_format_output` in `crates/server-less-core/src/lib.rs`.

v3 API: `Ctx::<data::JustLut<Val>>::new(&filter.lut, Vars::new([]))`, run via `filter.id.run(...)`, convert `serde_json::Value` → `Val` via `serde_json::from_value()`. No `RcIter`.

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
- No string-based code generation — use `quote!`.
- No hidden macro magic — generated code must be inspectable via `cargo expand`.
- No coupling to specific runtimes (tokio vs async-std) or dependency versions.
