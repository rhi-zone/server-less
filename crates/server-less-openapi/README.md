# server-less-openapi

[![crates.io](https://img.shields.io/crates/v/server-less-openapi.svg)](https://crates.io/crates/server-less-openapi)
[![docs.rs](https://img.shields.io/docsrs/server-less-openapi)](https://docs.rs/server-less-openapi)
[![License](https://img.shields.io/crates/l/server-less-openapi.svg)](https://github.com/rhi-zone/server-less/blob/master/LICENSE)

OpenAPI spec composition for [`server-less`](https://crates.io/crates/server-less). It provides `OpenApiBuilder` and the supporting spec types for merging OpenAPI specifications generated from multiple protocol sources into one document, plus an `OpenApiError` type for composition failures.

This crate is dependency-light (just `serde` / `serde_json`) and can be used standalone, or transitively via the `server-less` facade's `openapi` / `http` features.

## Example

```rust
use server_less::OpenApiBuilder;

let spec = OpenApiBuilder::new()
    .title("My API")
    .version("1.0.0")
    .merge(UserService::http_openapi_spec())
    .merge(OrderService::http_openapi_spec())
    .build()?;
```

Each `merge` folds another service's generated spec into the combined document, so a single OpenAPI file can describe an API stitched together from several independently-defined services.

## Documentation

- [Documentation site](https://rhi.zone/server-less/)
- [API docs (docs.rs)](https://docs.rs/server-less-openapi)
- [OpenAPI composition design](https://github.com/rhi-zone/server-less/blob/master/docs/design/openapi-composition.md)

See the [CHANGELOG](https://github.com/rhi-zone/server-less/blob/master/CHANGELOG.md).

## License

MIT — see [LICENSE](https://github.com/rhi-zone/server-less/blob/master/LICENSE).

---

Part of [RHI](https://rhi.zone/).
