---
layout: home
hero:
  name: Trellis
  text: Composable derive macros for Rust
  tagline: Structure for growing code
  actions:
    - theme: brand
      text: Get Started
      link: /guide/
    - theme: alt
      text: View on GitHub
      link: https://github.com/rhizome-lab/trellis
---

## Why "Trellis"?

A **trellis** is a lattice structure that supports climbing plants, giving them structure to grow upward while remaining flexible. The name reflects what this library does for Rust code:

- **Support structure** - Derive macros provide scaffolding for common patterns
- **Composability** - Like vines weaving through lattice, macros compose through attributes
- **Growth** - Start simple, add capabilities as needed

## Philosophy

Trellis macros are designed to be:

1. **Composable** - Mix and match capabilities via attributes
2. **Transparent** - Generated code is predictable and inspectable
3. **Minimal** - Only generate what you ask for
4. **Conventional** - Follow Rust ecosystem patterns

## Example

```rust
use trellis::prelude::*;

#[derive(Server)]
#[server(
    transport = "websocket",
    protocol = "json-rpc",
    middleware = [logging, auth]
)]
struct MyServer {
    // ...
}
```

## Part of Rhizome

Trellis is part of the [Rhizome](https://rhizome-lab.github.io/) ecosystem.
