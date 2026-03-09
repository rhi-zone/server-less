# Extension Coordination

How server-less macros coordinate with each other.

> **Note:** The sections below describe a planned derive-macro-based architecture (`#[derive(ServerCore, OpenApi, Serve)]`). The current implementation uses attribute macros on impl blocks. The coordination mechanism described here (naming convention, `Serve` wiring) reflects the planned design; the actual multi-protocol wiring today is done by blessed preset macros like `#[server]`. See [Blessed Presets](./blessed-presets.md).

## Multi-Protocol Attribute Stacking (Current)

Multiple protocol macros can be stacked on the same impl block. Each macro generates its own trait impls; the first macro in declaration order owns the impl block itself (subsequent macros strip the impl block from their output to avoid duplication):

```rust
#[http]
#[cli]
impl MyService {
    fn get_user(&self, user_id: u64) -> Option<User> { ... }
}
```

This generates both HTTP handlers and CLI subcommands from the same methods. The priority is determined by declaration order — `#[http]` owns the impl block, `#[cli]` appends only the CLI trait impls.

Blessed presets (`#[server]`, `#[program]`, `#[rpc]`, `#[tool]`) compose multiple macros internally using the same mechanism, so you rarely need to stack raw protocol macros manually.

## The Problem

Proc macros run independently and can't see each other's output. If you write:

```rust
#[derive(ServerCore, OpenApi, Metrics)]
struct MyServer;
```

Each derive runs separately. How do they wire together?

## The Serve Pattern

The `Serve` derive coordinates by parsing the derive list from syntax:

```rust
#[derive(ServerCore, OpenApi, Metrics, Serve)]
struct MyServer;
```

`Serve` sees `ServerCore`, `OpenApi`, `Metrics` in the attribute and generates wiring code:

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

## Type Safety

If you list a derive but don't actually include it, you get a compile error:

```rust
#[derive(ServerCore, OpenApi, Serve)]  // forgot to actually derive OpenApi
struct MyServer;

// Error: method `openapi` not found for `MyServer`
```

The type system enforces that listed derives are present.

## Extension Convention

Extensions generate a method with a known signature:

| Derive | Generated method |
|--------|------------------|
| `OpenApi` | `fn openapi() -> impl Layer` |
| `Metrics` | `fn metrics() -> impl Layer` |
| `FooExt` | `fn foo_ext() -> impl Layer` |

Convention: `{snake_case_derive_name}()` returns `impl Layer`.

Third-party crates follow this convention. `Serve` looks for `{snake_case}()` methods for any derive it sees in the list.

## Blessed Presets

The `Server` derive is a blessed preset that expands to multiple derives:

```rust
#[derive(Server)]
struct MyServer;

// Equivalent to:
#[derive(ServerCore, OpenApi, Metrics, HealthCheck, Serve)]
struct MyServer;
```

You can toggle features off:

```rust
#[derive(Server)]
#[server(openapi = false)]  // Server minus OpenApi
struct MyServer;
```

## Writing Extensions

To create a third-party extension:

1. Create a derive macro that generates a layer method:

```rust
// In your proc macro crate
#[proc_macro_derive(MyExtension)]
pub fn derive_my_extension(input: TokenStream) -> TokenStream {
    let name = /* parse struct name */;
    quote! {
        impl #name {
            pub fn my_extension() -> impl tower::Layer<...> {
                MyExtensionLayer::new()
            }
        }
    }
}
```

2. Users add it to their derive list:

```rust
#[derive(ServerCore, MyExtension, Serve)]
struct MyServer;
```

3. `Serve` sees `MyExtension` and generates `.layer(Self::my_extension())`.

## Tower Compatibility

All layers should be Tower-compatible:

```rust
impl<S> Layer<S> for MyExtensionLayer {
    type Service = MyExtensionService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        MyExtensionService::new(inner)
    }
}
```

This ensures extensions compose with the broader Rust ecosystem.
