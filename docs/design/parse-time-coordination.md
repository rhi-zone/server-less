# Parse-Time Coordination

Why `Serve` inspects the `#[derive(...)]` list at parse time instead of using a runtime registry.

## The Mechanism

`Serve` reads the derive list directly from the input token stream. When it sees:

```rust
#[derive(ServerCore, OpenApi, Metrics, Serve)]
struct MyServer;
```

it inspects the `#[derive(...)]` attribute, finds `OpenApi` and `Metrics`, and generates wiring calls for each. See [Extension Coordination](extension-coordination.md) for how the generated code looks.

## The Alternative: Runtime Registries

The `inventory` crate (and similar) let types register themselves into a global list at program startup via linker tricks. An extension crate could do:

```rust
// hypothetical - not how server-less works
inventory::submit!(LayerRegistration { name: "openapi", factory: || OpenApiLayer::new() });
```

and `Serve` would iterate that list at runtime to assemble the stack.

This is how some plugin systems work. We chose not to.

## Why Parse Time

**Errors surface at compile time, not startup.** If an extension is missing or misconfigured, the compiler reports it immediately. A runtime registry defers the failure to program launch - or worse, silently skips the layer if registration never ran.

**No hidden global state.** Runtime registries are global by nature. Two structs in the same binary that want different layer stacks would collide. Parse-time inspection is scoped to the struct being derived - each `#[derive(Serve)]` site sees exactly its own derive list.

**No ordering hazards.** Static initializers (the mechanism behind `inventory`) run in an unspecified order across compilation units. Getting the layer stack order wrong is silent and hard to debug. Parse-time inspection produces a deterministic order: the order the derives appear in source.

**No extra dependency.** `inventory` pulls in linkme or similar linker hacks. Parse-time inspection uses only `syn`, which proc macros already depend on.

**Readability.** The generated code (visible via `cargo expand`) contains explicit calls to named methods. There is no indirection through a registry lookup. What you read is what runs.

## The Tradeoff

Parse-time coordination is static. Extensions unknown at compile time cannot participate. A plugin loaded from a `.so` at runtime cannot add itself to the layer stack via this mechanism.

That tradeoff is intentional. Server-less targets the common case: a server whose composition is known when you write the code. Dynamic plugin loading is an escape hatch best handled at the Tower layer level directly, outside of the derive system.

This aligns with Rust's general philosophy: pay for dynamism only when you need it, and make the static case zero-cost and type-safe by default.

## Connection to the Naming Convention

Parse-time inspection makes the naming convention load-bearing. `Serve` converts each derive name to snake_case and generates `Self::{snake_case}()`. For `OpenApi` that is `Self::openapi()`. For a third-party `RateLimiter` that is `Self::rate_limiter()`.

The convention is what makes third-party extensions work without any registration step: follow the naming rule, and `Serve` will wire you in automatically. If the method is missing, the compiler reports an error with a precise span pointing at the derive list entry.

The naming convention is documented in [Extension Coordination](extension-coordination.md). The point here is that it only works *because* coordination happens at parse time - a runtime registry would not need or use the naming convention at all.
