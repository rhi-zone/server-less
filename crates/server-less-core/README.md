# server-less-core

[![crates.io](https://img.shields.io/crates/v/server-less-core.svg)](https://crates.io/crates/server-less-core)
[![docs.rs](https://img.shields.io/docsrs/server-less-core)](https://docs.rs/server-less-core)
[![License](https://img.shields.io/crates/l/server-less-core.svg)](https://github.com/rhi-zone/server-less/blob/master/LICENSE)

Runtime support crate for [`server-less`](https://crates.io/crates/server-less) — the foundational traits and types that the derive macros generate against.

> **Most users don't depend on this directly.** It's pulled in transitively when you depend on the [`server-less`](https://crates.io/crates/server-less) facade. Reach for it directly only when you're writing code that interoperates with generated output at the trait level.

## What's inside

- **Error model** — `ErrorCode` and the `IntoErrorCode` trait, plus `ErrorResponse`, `HttpStatusHelper`, and `HttpStatusFallback`. These back `#[derive(ServerlessError)]` and give every protocol projection a consistent way to map errors to HTTP statuses, exit codes, gRPC codes, and JSON-RPC codes.
- **Context extraction** — the `Context` extractor (and `WsSender` for WebSocket) used to pass request-scoped state into your methods.
- **CLI manual & output** — `CliManualNode` and the `cli_manual_to_json` / `cli_manual_to_text` helpers that build the whole-tree `--manual` reference surface, plus `cli_format_output` for `--json` / `--jq` formatting.
- **Mount & dispatch traits** — `CliSubcommand`, `McpNamespace`, `JsonRpcMount`, `WsMount`, and `HttpMount` for composing nested subcommands and routers.
- **Inference primitives** — `MethodInfo`, `ParamInfo`, `HttpMethod`, and path inference shared across projections.
- **Config module** — runtime support for `#[derive(Config)]` (TOML + environment layering).

## Features

| Feature | Enables |
|---------|---------|
| `cli` | CLI output formatting (`--json` / `--jq`) and manual helpers (clap, jaq) |
| `config` | Config loading support (TOML) |
| `http` | HTTP mount trait + OpenAPI types (axum) |
| `ws` | WebSocket support (futures, tokio, axum) |
| `mcp` | MCP namespace trait |
| `jsonrpc` | JSON-RPC mount trait |
| `jsonschema` | JSON Schema generation (schemars) |

These mirror the corresponding features on the `server-less` facade and are normally selected for you by it.

## Documentation

- [Documentation site](https://rhi.zone/server-less/)
- [API docs (docs.rs)](https://docs.rs/server-less-core)

See the [CHANGELOG](https://github.com/rhi-zone/server-less/blob/master/CHANGELOG.md).

## License

MIT — see [LICENSE](https://github.com/rhi-zone/server-less/blob/master/LICENSE).

---

Part of [RHI](https://rhi.zone/).
