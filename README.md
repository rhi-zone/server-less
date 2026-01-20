# Trellis

Composable derive macros for Rust.

## Why "Trellis"?

A **trellis** is a lattice structure that gardeners use to support climbing plants. It gives vines and creepers structure to grow upward while remaining flexible enough to adapt to any shape.

This library does the same for Rust code:

- **Support structure** - Derive macros provide scaffolding for common patterns (servers, configs, etc.)
- **Composability** - Like vines weaving through lattice, macros compose through attributes
- **Flexibility** - Configure exactly what you need, nothing more

## Usage

```rust
use trellis::prelude::*;

#[derive(Server)]
#[server(
    transport = "websocket",
    protocol = "json-rpc",
)]
struct MyServer {
    // ...
}
```

## Development

```bash
nix develop        # Enter dev shell
cargo build        # Build all crates
cargo test         # Run tests
```

## Documentation

```bash
cd docs
bun install
bun run dev        # Local docs server
```

## Part of Rhizome

Trellis is part of the [Rhizome](https://rhizome-lab.github.io/) ecosystem - tools for programmable creativity.
