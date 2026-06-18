# server-less-parse

[![crates.io](https://img.shields.io/crates/v/server-less-parse.svg)](https://crates.io/crates/server-less-parse)
[![docs.rs](https://docs.rs/server-less-parse/badge.svg)](https://docs.rs/server-less-parse)
[![license](https://img.shields.io/crates/l/server-less-parse.svg)](https://github.com/rhi-zone/server-less/blob/master/LICENSE)

Internal parsing utilities for the server-less proc macros.

> **Internal crate — no stability guarantees.** This crate exists only to support
> the [`server-less`](https://crates.io/crates/server-less) proc macros, and is
> published solely because path dependencies are disallowed. Its public surface is
> typed in terms of [`syn`](https://crates.io/crates/syn) /
> [`proc-macro2`](https://crates.io/crates/proc-macro2) and may change in **any**
> release, including patch releases.
> **Depend on [`server-less`](https://crates.io/crates/server-less) instead.**

It extracts the rich, compile-time representation (`MethodInfo`, `ParamInfo`,
and related types) from impl blocks, retaining full `syn` AST nodes so the
macros can generate accurate token output.

## Changelog

See [CHANGELOG.md](https://github.com/rhi-zone/server-less/blob/master/CHANGELOG.md).

---

Part of [RHI](https://rhi.zone/).
