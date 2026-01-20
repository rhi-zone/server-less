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

## Planned Macros

### Server Setup (`trellis-server`)

```rust
// Blessed preset - batteries included
#[derive(Server)]
struct MyServer;

// À la carte with config
#[derive(ServerCore, OpenApi, Serve)]
#[server(
    transport = "websocket" | "tcp" | "unix",
    protocol = "json-rpc" | "capnproto" | "msgpack" | "custom",
)]
struct MyServer;
```

### Configuration Loading

```rust
#[derive(Config)]
#[config(
    sources = ["env", "file:config.toml", "args"],
    prefix = "MY_APP",
)]
struct AppConfig {
    #[config(env = "PORT", default = 8080)]
    port: u16,
}
```

### Builder Pattern

```rust
#[derive(Builder)]
#[builder(style = "owned" | "borrowed" | "async")]
struct Request {
    #[builder(required)]
    url: String,
    #[builder(default = "GET")]
    method: String,
}
```

### Future Ideas

- `#[derive(Cli)]` - clap-style CLI generation
- `#[derive(Api)]` - OpenAPI/REST endpoint generation
- `#[derive(Event)]` - Event sourcing patterns
- `#[derive(Query)]` - Type-safe query builders

## Implementation Notes

### Proc Macro Crate Structure

Each macro category gets its own crate for:
- Faster incremental compilation
- Independent versioning
- Clear dependency boundaries

The main `trellis` crate re-exports everything:
```rust
pub use trellis_server::Server;
pub use trellis_config::Config;
// etc.
```

### Extension Coordination (The Serve Pattern)

Proc macros run independently and can't see each other's output. Coordination works via the `Serve` derive:

```rust
#[derive(ServerCore, OpenApi, Metrics, Serve)]
struct MyServer;
```

`Serve` parses the derive list from syntax, then generates wiring code:
```rust
// Serve generates:
impl MyServer {
    pub async fn serve(self) {
        self.into_service()           // from ServerCore
            .layer(Self::openapi())   // from OpenApi
            .layer(Self::metrics())   // from Metrics
            .run()
            .await
    }
}
```

**Type safety**: If you list `OpenApi` in derives but don't actually derive it → compile error ("method `openapi` not found").

**Extension convention**: Extensions generate a method with a known signature:
- `#[derive(OpenApi)]` → `fn openapi() -> impl Layer`
- `#[derive(Metrics)]` → `fn metrics() -> impl Layer`
- `#[derive(FooExt)]` → `fn foo_ext() -> impl Layer` (convention: snake_case of derive name)

Third-party crates just follow the convention. `Serve` knows to look for `{snake_case}_layer()` methods for any derive it sees.

### Tower Compatibility

Server macros should generate Tower-compatible services where possible:
```rust
impl<S> Layer<S> for GeneratedMiddleware {
    type Service = GeneratedService<S>;
    // ...
}
```

### Error Handling

Use `syn::Error` with proper spans:
```rust
return Err(syn::Error::new_spanned(
    attr,
    "transport must be one of: websocket, tcp, unix"
));
```

## Development

```bash
nix develop              # Enter dev shell
cargo build              # Build all crates
cargo test               # Run tests
cargo expand             # Inspect macro expansion (install cargo-expand first)
```

### Testing Macros

```rust
#[test]
fn test_server_expansion() {
    let input: DeriveInput = parse_quote! {
        #[server(transport = "websocket")]
        struct MyServer;
    };

    let output = expand_server(input);
    // Assert on generated tokens
}
```

Use `trybuild` for compile-fail tests:
```rust
#[test]
fn ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
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
