# Design Docs

Architecture decisions and design philosophy for server-less.

## Core Design

- [Impl-First Design](./impl-first.md) — Protocol projections, naming conventions, return types
- [Inference vs Configuration](./inference-vs-configuration.md) — Full reference for all inference rules and overrides
- [Param Attributes](./param-attributes.md) — `#[param]` cross-protocol design, clap alignment, positional ordering
- [Error Mapping](./error-mapping.md) — `#[derive(ServerlessError)]` inference and protocol dispatch

## Composition & Coordination

- [Extension Coordination](./extension-coordination.md) — How derives compose via `Serve`
- [Parse-Time Coordination](./parse-time-coordination.md) — Why compile-time inspection over runtime registries
- [Protocol Naming](./protocol-naming.md) — `PascalCase` derive → `snake_case` method convention
- [Blessed Presets](./blessed-presets.md) — `#[server]`, `#[rpc]`, `#[tool]`, `#[program]` presets

## Feature-Specific

- [CLI Output Formatting](./cli-output-formatting.md) — Display default, `--json`/`--jq`/`--output-schema`
- [Route & Response Attributes](./route-response-attrs.md) — `#[route]` and `#[response]` HTTP overrides
- [Mount Points](./mount-points.md) — Nested subcommand composition via `&T` return types
- [OpenAPI Composition](./openapi-composition.md) — Multi-protocol OpenAPI spec composition
- [Config Management](./config.md) — `#[derive(Config)]`, config sources, and the generated `config` subcommand
- [Application Metadata](./app-metadata.md) — `#[app]` for name, description, version, homepage across all protocols

## Process

- [Open Questions](./open-questions.md) — Unresolved design questions
- [Iteration Log](./iteration-log.md) — Development history
- [Implementation Notes](./implementation-notes.md) — Early implementation snapshot
