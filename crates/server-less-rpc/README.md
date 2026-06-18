# server-less-rpc

[![crates.io](https://img.shields.io/crates/v/server-less-rpc.svg)](https://crates.io/crates/server-less-rpc)
[![docs.rs](https://docs.rs/server-less-rpc/badge.svg)](https://docs.rs/server-less-rpc)
[![license](https://img.shields.io/crates/l/server-less-rpc.svg)](https://github.com/rhi-zone/server-less/blob/master/LICENSE)

Internal JSON-RPC-style dispatch codegen for the server-less proc macros.

> **Internal crate — no stability guarantees.** This crate exists only to support
> the [`server-less`](https://crates.io/crates/server-less) proc macros, and is
> published solely because path dependencies are disallowed. Its public surface is
> typed in terms of [`proc-macro2`](https://crates.io/crates/proc-macro2) token
> streams and may change in **any** release, including patch releases.
> **Depend on [`server-less`](https://crates.io/crates/server-less) instead.**

It provides the shared JSON-RPC-like param-extraction and dispatch code
generation used by the MCP, WebSocket, and JSON-RPC macros — extracting params
from a JSON args object, calling the method, and serializing the result back.

## Changelog

See [CHANGELOG.md](https://github.com/rhi-zone/server-less/blob/master/CHANGELOG.md).

---

Part of [RHI](https://rhi.zone/).
