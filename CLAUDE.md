# CLAUDE.md

Behavioral rules for Claude Code in the trellis repository.

## Project Overview

Trellis is a collection of composable derive macros for common Rust patterns. The name comes from garden trellises - lattice structures that support climbing plants, providing structure while remaining flexible.

**Origin story:** While building Lotus (an object store), we wanted derive macros for server setup - websocket servers, JSON-RPC, Cap'n Proto, etc. Rather than building one-off solutions, we decided to create a repo for "random derive macros for the silliest stuff ever" - but designed properly for composability.

## Philosophy

### Composability First

The core insight: macros should compose like Tower's Service/Layer pattern. Each macro adds a capability, and they stack cleanly:

```rust
#[derive(Server, Logging, Auth, RateLimit)]
#[server(transport = "websocket", protocol = "json-rpc")]
#[logging(level = "info")]
#[auth(method = "jwt")]
#[rate_limit(requests = 100, per = "minute")]
struct MyServer {
    // ...
}
```

This should "just work" - no conflicts, predictable ordering, each macro does its job.

### Configuration via Attributes

Prefer attribute-based configuration over separate config files or builder patterns:
- Attributes are colocated with the type
- IDE support for autocomplete
- Compile-time validation
- Self-documenting

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
#[derive(Server)]
#[server(
    transport = "websocket" | "tcp" | "unix",
    protocol = "json-rpc" | "capnproto" | "msgpack" | "custom",
    middleware = [logging, auth, rate_limit],
)]
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
