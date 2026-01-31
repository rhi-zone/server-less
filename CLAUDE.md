# CLAUDE.md

Behavioral rules for Claude Code in the server-less repository.

## Project Overview

**server-less** is a collection of composable derive macros for Rust. The name is literal: write less server code. It's about minimizing boilerplate while keeping full control.

**Origin story:** While building Lotus (an object store), we wanted derive macros for server setup - websocket servers, JSON-RPC, Cap'n Proto, etc. Rather than building one-off solutions, we decided to create a repo for "random derive macros for the silliest stuff ever" - but designed properly for composability. The name "server-less" emphasizes the pragmatic goal: write less code to build servers.

## Philosophy

### Minimize Barrier to Entry

The primary goal is making the simple case trivially simple. "I just want a server" should be:

```rust
#[derive(Server)]
struct MyServer;
```

Yes, this restricts possibilities compared to hand-written code. That's the trade-off. The question is: how do we minimize that impact while maximizing accessibility?

### Progressive Disclosure

Complexity should only appear when you need it. The zero-config case doesn't even hint at the options:

```rust
// Level 1: Just works
#[derive(Server)]
struct MyServer;

// Level 2: Toggle features
#[derive(Server)]
#[server(openapi = false)]
struct MyServer;

// Level 3: Fine-tune
#[derive(Server)]
#[server(openapi(path = "/docs", hidden = [internal_method]))]
struct MyServer;

// Level 4: Escape hatch - drop to manual code
```

You discover options when you need them, not before.

### Gradual Refinement

Like gradual typing: start with the simple version, incrementally add control as requirements evolve. You shouldn't have to rewrite everything when you need one custom behavior.

Don't like how server-less does auth? Don't use `#[derive(Auth)]`, write your own Tower layer - it still composes with `#[derive(Server)]`. The escape hatch is granular, not all-or-nothing.

### Two-Tier Design: Blessed Presets vs À La Carte

**Blessed preset** - just works, batteries included:
```rust
#[derive(Server)]  // includes: ServerCore + OpenApi + Metrics + HealthCheck + Serve
struct MyServer;
```

**À la carte** - full control over composition:
```rust
#[derive(ServerCore, OpenApi, Metrics, Serve)]  // explicit, no HealthCheck
struct MyServer;
```

**Toggle within blessed**:
```rust
#[derive(Server)]
#[server(openapi = false)]  // blessed preset minus OpenApi
struct MyServer;
```

This gives progressive disclosure:
1. `#[derive(Server)]` - blessed, zero config
2. `#[derive(Server)] #[server(x = false)]` - blessed minus some
3. `#[derive(ServerCore, X, Y, Serve)]` - explicit composition
4. Manual Tower code - full escape hatch

### Third-Party Extensions

Extensions are separate derives that compose with core:
```rust
#[derive(ServerCore, OpenApi, Anubis, Serve)]  // Anubis from server-less-anubis crate
struct MyServer;
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
- [Impl-First Design](docs/design/impl-first.md) - Protocol projections and conventions
- [Extension Coordination](docs/design/extension-coordination.md) - How derives compose

## Development

```bash
nix develop        # Enter dev shell
cargo build        # Build all crates
cargo test         # Run tests
cargo expand       # Inspect macro expansion
```

## Part of RHI

Server-less is part of the [Rhizome](https://rhi.zone/) ecosystem. Related projects:
- **Lotus** - Object store (uses server-less for server setup)
- **Spore** - Lua runtime with LLM integration
- **Hypha** - Async runtime primitives

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
| Type stubs | `typescript`, `python` | No |

**Always available:** `ServerlessError` derive (commonly needed, zero deps).

## Workflow

**Batch cargo commands** to minimize round-trips:
```bash
cargo clippy --all-targets --all-features -- -D warnings && cargo test
```
After editing multiple files, run the full check once — not after each edit. Formatting is handled automatically by the pre-commit hook (`cargo fmt`).

**When making the same change across multiple crates**, edit all files first, then build once.

**Use `normalize view` for structural exploration:**
```bash
~/git/rhizone/normalize/target/debug/normalize view <file>    # outline with line numbers
~/git/rhizone/normalize/target/debug/normalize view <dir>     # directory structure
```

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

## Core Rules

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

## Negative Constraints

Do not:
- Announce actions ("I will now...") - just do them
- Leave work uncommitted
- Generate code that isn't explicitly requested
- Hide complexity in macro magic
- Break when composed with other macros
- Assume specific runtime environments (tokio vs async-std, etc.)
- Couple to specific versions of dependencies
- Use string-based code generation (use `quote!`)
- Use path dependencies in Cargo.toml - causes clippy to stash changes across repos
- Use `--no-verify` - fix the issue or fix the hook
- Assume tools are missing - check if `nix develop` is available for the right environment
