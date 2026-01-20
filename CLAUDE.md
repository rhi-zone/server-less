# CLAUDE.md

Behavioral rules for Claude Code in the trellis repository.

## Project Overview

Trellis is a collection of composable derive macros for common Rust patterns. The name comes from garden trellises - lattice structures that support climbing plants, providing structure while remaining flexible.

**Origin story:** While building Lotus (an object store), we wanted derive macros for server setup - websocket servers, JSON-RPC, Cap'n Proto, etc. Rather than building one-off solutions, we decided to create a repo for "random derive macros for the silliest stuff ever" - but designed properly for composability.

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

Don't like how trellis does auth? Don't use `#[derive(Auth)]`, write your own Tower layer - it still composes with `#[derive(Server)]`. The escape hatch is granular, not all-or-nothing.

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
#[derive(ServerCore, OpenApi, Anubis, Serve)]  // Anubis from trellis-anubis crate
struct MyServer;
```

The ecosystem has great solutions (rate limiting, auth, observability, bot protection). Trellis should make them accessible via derives, not reinvent them. Popular extensions can graduate to "blessed" status over time.

### "Silly but Proper"

The macros might be for simple things, but the implementation should be solid:
- Proper error messages with span information
- Generated code is readable (run `cargo expand` to verify)
- No hidden magic - what you see is what you get
- Tested extensively

## Structure

```
trellis/
├── crates/
│   ├── trellis/              # Main crate (re-exports all macros)
│   ├── trellis-derive/       # Proc macro implementations
│   ├── trellis-server/       # Server-related macros
│   └── trellis-*/            # Other macro categories
└── docs/                     # VitePress documentation
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

## Part of Rhizome

Trellis is part of the [Rhizome](https://rhizome-lab.github.io/) ecosystem. Related projects:
- **Lotus** - Object store (uses trellis for server setup)
- **Spore** - Lua runtime with LLM integration
- **Hypha** - Async runtime primitives

## Core Rules

- Macros are opt-in, not opt-out
- Generated code must be readable
- Error messages must be helpful (with spans!)
- Document all attributes in both code and docs
- Test macro expansion AND generated behavior

## Negative Constraints

Do not:
- Generate code that isn't explicitly requested
- Hide complexity in macro magic
- Break when composed with other macros
- Assume specific runtime environments (tokio vs async-std, etc.)
- Couple to specific versions of dependencies
- Use string-based code generation (use `quote!`)
