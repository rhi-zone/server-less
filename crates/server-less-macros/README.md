# server-less-macros

[![crates.io](https://img.shields.io/crates/v/server-less-macros.svg)](https://crates.io/crates/server-less-macros)
[![docs.rs](https://img.shields.io/docsrs/server-less-macros)](https://docs.rs/server-less-macros)
[![License](https://img.shields.io/crates/l/server-less-macros.svg)](https://github.com/rhi-zone/server-less/blob/master/LICENSE)

Proc-macro implementation crate for [`server-less`](https://crates.io/crates/server-less).

> **Don't depend on this crate directly.** It's the procedural-macro backend, pulled in automatically by the [`server-less`](https://crates.io/crates/server-less) facade. Depend on `server-less` instead — it re-exports every macro here alongside the runtime support they generate against, gated behind the matching feature flags. Using `server-less-macros` on its own gives you the macros without the traits and types their output requires.

## What it provides

The attribute and derive macros that turn an impl block into protocol projections:

- **Runtime protocols** — `#[http]`, `#[cli]`, `#[mcp]`, `#[ws]`, `#[jsonrpc]`, `#[graphql]`.
- **Schema generators** — `#[grpc]`, `#[capnp]`, `#[thrift]`, `#[smithy]`, `#[connect]`.
- **Spec & doc generators** — `#[openapi]`, `#[openrpc]`, `#[asyncapi]`, `#[jsonschema]`, `#[markdown]`.
- **Blessed presets** — `#[server]`, `#[rpc]`, `#[tool]`, `#[program]`.
- **Coordination & metadata** — `#[serve]`, `#[route]`, `#[response]`, `#[param]`, `#[app]`.
- **Derives** — `#[derive(ServerlessError)]`, `#[derive(HealthCheck)]`, and config support for `#[derive(Config)]`.

Each macro lives behind a feature flag mirroring the `server-less` facade, so you only compile the projections you use.

## Documentation

- [Documentation site](https://rhi.zone/server-less/)
- [API docs (docs.rs)](https://docs.rs/server-less-macros)

See the [CHANGELOG](https://github.com/rhi-zone/server-less/blob/master/CHANGELOG.md).

## License

MIT — see [LICENSE](https://github.com/rhi-zone/server-less/blob/master/LICENSE).

---

Part of [RHI](https://rhi.zone/).
