# Application Metadata (`#[app]`)

How top-level application name, description, version, and homepage are expressed and consumed across protocols.

## The Problem

Every protocol that exposes an application identity needs the same basic information:

- **OpenAPI** needs `info.title`, `info.description`, `info.version`
- **CLI** needs an application name (help header) and `--version` output
- **Config** needs a name for the default config file path (`{name}.toml`)
- **OpenRPC**, **MCP**, etc. need similar metadata in their respective specs

Before `#[app]`, this was inconsistent: `#[program(name, about, version)]` covered CLI, HTTP hardcoded the struct name as the OpenAPI title, and nothing else had a way to express metadata at all.

## The `#[app]` Attribute

A shared, protocol-neutral attribute that any impl block can carry:

```rust
#[app(
    name = "myapp",
    description = "Does the thing",
    version = "2.1.0",
    homepage = "https://myapp.example.com",
)]
#[server]
impl MyApi {
    fn list_items(&self) -> Vec<Item> { ... }
}
```

`#[app]` is consumed by all derives on the same impl block. It is not a macro that generates code on its own — it is metadata that other macros read.

## Fields

| Field | Type | Default | Effect |
|-------|------|---------|--------|
| `name` | string | inferred from struct name (kebab-case) | App name used in config file path, CLI header, spec titles |
| `description` | string | none | Human-readable description; CLI `--help` body, OpenAPI `info.description`, etc. |
| `version` | string or `false` | `env!("CARGO_PKG_VERSION")` | Version string; powers `--version` flag; `false` disables version entirely |
| `homepage` | string | none | URL; appears in OpenAPI `info.contact.url`, OpenRPC `info.contact`, etc. |

## Name Inference

If `name` is not provided, it is inferred from the struct name using these casing rules:

| Context | Rule | Example (`MyHttpServer`) |
|---------|------|--------------------------|
| Config file path | kebab-case | `my-http-server.toml` |
| CLI app name | kebab-case | `my-http-server` |
| OpenAPI `info.title` | Title Case (space-separated) | `My Http Server` |
| `--version` output prefix | kebab-case | `my-http-server 1.0.0` |

## Version Handling

The default version is `env!("CARGO_PKG_VERSION")`, resolved at compile time from the crate's `Cargo.toml`. An explicit `version = "..."` attribute overrides this.

When a version is set (which it always is, unless explicitly disabled), clap automatically generates `-V`/`--version` for CLI projections. To disable:

```rust
#[app(version = false)]   // no version anywhere — no --version flag, no version in specs
#[server]
impl MyApi { ... }
```

To suppress only for CLI without removing version from specs, use the CLI-specific opt-out:

```rust
#[app(name = "myapp")]
#[cli(no_version)]         // --version flag suppressed in CLI only
#[server]
impl MyApi { ... }
```

## Per-Preset Override

All blessed presets accept the same keys inline, as a shorthand for `#[app]`:

```rust
#[server(name = "myapp", description = "Does the thing")]
impl MyApi { ... }
```

This is equivalent to:

```rust
#[app(name = "myapp", description = "Does the thing")]
#[server]
impl MyApi { ... }
```

When both are present, per-preset values take precedence over `#[app]` for that preset only. This allows a monorepo with multiple impl blocks to share a base `#[app]` and override per-service as needed.

## Supersedes Previous Attributes

`#[app]` supersedes the existing per-macro metadata attributes:

| Old | New |
|-----|-----|
| `#[program(name = "...")]` | `#[app(name = "...")]` |
| `#[program(about = "...")]` | `#[app(description = "...")]` |
| `#[program(version = "...")]` | `#[app(version = "...")]` |
| `#[cli(version = "...")]` | `#[app(version = "...")]` |

The old attributes are deprecated but kept for backwards compatibility until a major version bump removes them.

## What Consumes `#[app]`

| Protocol / Feature | Field(s) used |
|--------------------|---------------|
| OpenAPI `info` | `name` → title, `description`, `version`, `homepage` → contact URL |
| CLI help header | `name`, `description` |
| CLI `--version` | `version` |
| Config file default path | `name` → `{name}.toml` |
| OpenRPC `info` | `name`, `description`, `version`, `homepage` |
| MCP server info | `name`, `description`, `version` |
| AsyncAPI `info` | `name`, `description`, `version` |

## Open Questions

- **Acronym handling in title case**: `MyHTTPServer` → `My Http Server` or `My HTTP Server`? Simple word-splitting on case boundaries produces the former; recognizing common acronyms would produce the latter but requires a list. Start simple, revisit if users hit it.
- **Multiple impl blocks**: If the same binary has two services with different `#[app]` attributes, each owns its own metadata independently. Is there a use case for a binary-level `#[app]` that all services inherit? Probably not — keep it per-impl.
