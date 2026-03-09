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

## Core Rules

**Note things down immediately — no deferral:**
- Problems, tech debt, issues → TODO.md now, in the same response
- Design decisions, key insights → docs/ or CLAUDE.md
- Future/deferred scope → TODO.md **before** writing any code, not after
- **Every observed problem → TODO.md. No exceptions.** Code comments and conversation mentions are not tracked items. If you write a TODO comment in source, the next action is to open TODO.md and write the entry.

**Conversation is not memory.** Anything said in chat evaporates at session end. If it implies future behavior change, write it to CLAUDE.md or a memory file immediately — or it will not happen.

**Warning — these phrases mean something needs to be written down right now:**
- "I won't do X again" / "I'll remember to..." / "I've learned that..."
- "Next time I'll..." / "From now on I'll..."
- Any acknowledgement of a recurring error without a corresponding CLAUDE.md or memory edit

**Triggers:** User corrects you, 2+ failed attempts, "aha" moment, framework quirk discovered → document before proceeding.

**When the user corrects you:** Ask what rule would have prevented this, and write it before proceeding. **"The rule exists, I just didn't follow it" is never the diagnosis** — a rule that doesn't prevent the failure it describes is incomplete; fix the rule, not your behavior.

**Something unexpected is a signal, not noise.** Surprising output, anomalous numbers, files containing what they shouldn't — stop and ask why before continuing. Don't accept anomalies and move on.

**Do the work properly.** Don't leave workarounds or hacks undocumented. When asked to analyze X, actually read X — don't synthesize from conversation.

**Macro-specific rules:**
- Macros are opt-in, not opt-out
- Generated code must be readable
- Error messages must be helpful (with spans!)
- Document all attributes in both code and docs
- Test macro expansion AND generated behavior

## Design Principles

**Unify, don't multiply.** One interface for multiple cases > separate interfaces. Plugin systems > hardcoded switches.

**Simplicity over cleverness.** HashMap > inventory crate. OnceLock > lazy_static. Functions > traits until you need the trait. Use ecosystem tooling over hand-rolling.

**Explicit over implicit.** Log when skipping. Show what's at stake before refusing.

**Separate niche from shared.** Don't bloat shared config with feature-specific data. Use separate files for specialized data.

## Workflow

**Batch cargo commands** to minimize round-trips:
```bash
cargo clippy --all-targets --all-features -- -D warnings && cargo test
```
After editing multiple files, run the full check once — not after each edit. Formatting is handled automatically by the pre-commit hook (`cargo fmt`).

**When making the same change across multiple crates**, edit all files first, then build once.

**Minimize file churn.** When editing a file, read it once, plan all changes, and apply them in one pass. Avoid read-edit-build-fail-read-fix cycles by thinking through the complete change before starting.

**Use `normalize view` for structural exploration:**
```bash
~/git/rhizone/normalize/target/debug/normalize view <file>    # outline with line numbers
~/git/rhizone/normalize/target/debug/normalize view <dir>     # directory structure
```

**Always commit completed work.** After tests pass, commit immediately — don't wait to be asked. When a plan has multiple phases, commit after each phase passes. Do not accumulate changes across phases. Uncommitted work is lost work.

## Context Management

**Use subagents to protect the main context window.** For broad exploration or mechanical multi-file work, delegate to an Explore or general-purpose subagent rather than running searches inline. The subagent returns a distilled summary; raw tool output stays out of the main context.

Rules of thumb:
- Research tasks (investigating a question, surveying patterns) → subagent; don't pollute main context with exploratory noise
- Searching >5 files or running >3 rounds of grep/read → use a subagent
- Codebase-wide analysis (architecture, patterns, cross-file survey) → always subagent
- Mechanical work across many files (applying the same change everywhere) → parallel subagents
- Single targeted lookup (one file, one symbol) → inline is fine

## Session Handoff

Use plan mode as a handoff mechanism when:
- A task is fully complete (committed, pushed, docs updated)
- The session has drifted from its original purpose
- Context has accumulated enough that a fresh start would help

**For handoffs:** enter plan mode, write a short plan pointing at TODO.md, and ExitPlanMode. **Do NOT investigate first** — the session is context-heavy and about to be discarded. The fresh session investigates after approval.

**For mid-session planning** on a different topic: investigating inside plan mode is fine — context isn't being thrown away.

Before the handoff plan, update TODO.md and memory files with anything worth preserving.

## Commit Convention

Use conventional commits: `type(scope): message`

Types:
- `feat` - New feature
- `fix` - Bug fix
- `refactor` - Code change that neither fixes a bug nor adds a feature
- `docs` - Documentation only
- `chore` - Maintenance (deps, CI, etc.)
- `test` - Adding or updating tests

Scope is optional but recommended for multi-crate repos.

## Negative Constraints

Do not:
- Announce actions ("I will now...") - just do them
- Leave work uncommitted
- Use interactive git commands (`git add -p`, `git add -i`, `git rebase -i`) — these block on stdin and hang in non-interactive shells; stage files by name instead
- Generate code that isn't explicitly requested
- Hide complexity in macro magic
- Break when composed with other macros
- Assume specific runtime environments (tokio vs async-std, etc.)
- Couple to specific versions of dependencies
- Use string-based code generation (use `quote!`)
- Use path dependencies in Cargo.toml - causes clippy to stash changes across repos
- Use `--no-verify` - fix the issue or fix the hook
- Assume tools are missing - check if `nix develop` is available for the right environment
