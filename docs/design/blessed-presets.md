# Blessed Presets

Two tiers for two different needs: get started in one line, or compose exactly what you want.

## The Two Tiers

**Blessed preset** - batteries included, zero decisions:

```rust
#[server]
impl MyApi {
    fn list_items(&self) -> Vec<Item> { ... }
    fn create_item(&self, name: String) -> Item { ... }
}
```

`#[server]` expands to `#[http]` + `#[serve(http)]`. You get HTTP routes, a health check, and an OpenAPI spec. No configuration required.

**A la carte** - explicit composition, full control:

```rust
#[http(prefix = "/api/v1")]
#[serve(http, health = "/healthz")]
impl MyApi {
    fn list_items(&self) -> Vec<Item> { ... }
    fn create_item(&self, name: String) -> Item { ... }
}
```

Each macro is listed explicitly. You see exactly what you get. Adding `#[openapi]` is a deliberate act, not a hidden default.

The blessed presets in this codebase:

| Preset | Expands to | Use case |
|--------|-----------|----------|
| `#[server]` | `#[http]` + `#[serve(http)]` | HTTP REST API |
| `#[rpc]` | `#[jsonrpc]` + `#[openrpc]` + `#[serve(jsonrpc)]` | JSON-RPC service |
| `#[tool]` | `#[mcp]` + `#[jsonschema]` | MCP tools for LLMs |
| `#[program]` | `#[cli]` + `#[markdown]` | CLI application |

## Toggles Bridge the Tiers

The blessed presets aren't all-or-nothing. Toggles let you adjust without dropping to a la carte:

```rust
// Level 1: Zero config
#[server]
impl MyApi { ... }

// Level 2: Turn off a component
#[server(openapi = false)]
impl MyApi { ... }

// Level 3: Configure a component
#[server(prefix = "/api/v1")]
impl MyApi { ... }

// Level 4: Drop to a la carte
#[http(prefix = "/api/v1", openapi = false)]
#[serve(http)]
impl MyApi { ... }
```

You don't have to jump straight from "just works" to "write everything manually." The toggles let you incrementally adjust. You only learn what you need, when you need it.

## Why Both Tiers Exist

The simple case should be trivially simple. "I want an HTTP server" shouldn't require understanding Tower layers, axum routers, or how `#[http]` and `#[serve]` wire together.

But the escape hatch matters too. A preset that can't be customized is a trap. When you need specific behavior - a non-standard health check path, OpenAPI disabled for internal services, a custom prefix - you shouldn't have to throw away the preset and start from scratch.

The two tiers give you a choice at each step:
- Stay in the preset and use toggles for most customizations
- Drop to a la carte when you need full control over composition
- Write Tower layers directly when you need to escape entirely

The transition is granular. Switching from `#[server]` to `#[http]` + `#[serve]` is mechanical, not a rewrite.

## Third-Party Extensions

Extensions compose naturally with both tiers.

With a la carte, extensions appear in the list alongside built-in derives. `#[serve]` sees everything in the attribute list and wires it together:

```rust
#[http]
#[anubis]          // from server-less-anubis crate (bot protection)
#[serve(http, anubis)]
impl MyApi { ... }
```

With blessed presets, extensions slot in next to the preset:

```rust
#[server]
#[anubis]
impl MyApi { ... }
```

The extension convention is simple: any derive that follows the `{snake_case_name}() -> impl Layer` convention works automatically. `#[serve]` discovers extensions by looking for that method signature. See [Extension Coordination](extension-coordination.md) for the full protocol.

This means third-party extensions work at both tiers without special handling. Popular extensions can graduate to blessed status over time - they become toggles within the preset rather than explicit adds.

## The Serde Parallel

Serde is the model. `#[derive(Serialize, Deserialize)]` is the blessed preset: one line, sensible defaults, works for most types immediately. Behind it is a full customization surface: `#[serde(rename = "...")]`, `#[serde(skip)]`, `#[serde(flatten)]`, custom serializers, multiple format support.

The derive macro is the interface, not a straitjacket. Defaults get you 80% of the way. The other 20% is there when you need it, and it composes cleanly with the defaults rather than replacing them.

Server-less follows the same model. `#[server]` is the derive. The toggles are the `#[serde(...)]` attributes. The a la carte tier is writing a custom serializer: you opt into full control when you need it, not before.
