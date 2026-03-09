# Config Management

How `#[derive(Config)]` works, how it links to servers, and what the generated `config` subcommand looks like.

## The Problem

`#[derive(Server)]` promises zero config — "just works" out of the box. But "just works" requires sensible defaults *and* a way to override them. Without explicit config management, users resort to hand-rolling env var reads, scattering `std::env::var()` calls, and writing their own introspection tooling.

`#[derive(Config)]` eliminates that boilerplate. It projects a config struct onto config sources (env vars, files, defaults) and generates a high-quality `config` subcommand for discoverability and introspection.

## Shape

Config lives in a dedicated struct, separate from the server struct:

```rust
#[derive(Config)]
struct MyConfig {
    #[param(env = "PORT", default = 3000, help = "Port to listen on")]
    port: u16,

    #[param(env = "LOG_LEVEL", default = "info", help = "Log level (trace/debug/info/warn/error)")]
    log_level: String,

    #[param(env = "DATABASE_URL", help = "Postgres connection string")]
    database_url: String,
}

#[derive(Server)]
#[server(config = MyConfig)]
struct MyServer;
```

**Why a separate struct?** The server struct is about behavior (methods, routes). The config struct is about settings (port, log level, timeouts). Mixing them conflates two concerns. Keeping them separate also means each `#[derive(Server)]` can have its own config with no spooky action at a distance — multiple servers in one binary each own their config cleanly.

## Config Sources and Precedence

Default precedence (highest to lowest):

1. CLI flags (when running the `config set` subcommand or passing overrides)
2. Environment variables
3. Config file (TOML by default)
4. Compiled-in defaults (`default = ...` in `#[param]`)

The precedence order is configurable:

```rust
#[derive(Config)]
#[config(precedence = [env, file, default])]  // skip CLI overrides
struct MyConfig { ... }
```

## The `config` Subcommand

When a server has a linked config, a `config` subcommand is generated automatically. It has four sub-subcommands, following the same pattern as normalize's config command:

### `config show [--section <path>]`

Displays *all available config fields* with their descriptions, types, defaults, and current values — including fields not currently set. Unset fields are shown as comments. This is the discoverability win: users can see everything that's configurable without reading source code or docs.

```
$ myapp config show
# Port to listen on
# type: u16
port = 3000                    # default

# Log level (trace/debug/info/warn/error)
# type: String
log_level = "debug"            # env: LOG_LEVEL

# Postgres connection string
# type: String
# database_url = (unset)
```

Supports dotted-path sections for nested configs:

```
$ myapp config show --section database
```

### `config schema`

Emits the full JSON Schema for the config struct. Useful for IDE integrations, external tooling, and documentation generation.

```
$ myapp config schema
{ "$schema": "...", "type": "object", "properties": { ... } }
```

### `config validate`

Validates the current config (all sources merged) against the schema. Reports success or a list of errors.

```
$ myapp config validate
✓ Config valid

$ myapp config validate
✗ Config invalid:
  - database_url: required field is unset
```

### `config set <key> <value> [--dry-run]`

Sets a value in the config file by dotted key path. Auto-coerces values (`true`/`false` → bool, integers, floats, else string). Validates the result against the schema after writing.

`--dry-run` previews the change without writing:

```
$ myapp config set port 8080 --dry-run
Would set port: 3000 → 8080
```

## Blessed Preset Integration

`#[derive(Server)]` includes config management when a config is linked:

```rust
// Blessed — config subcommand generated automatically
#[derive(Server)]
#[server(config = MyConfig)]
struct MyServer;

// A la carte — explicit
#[derive(ServerCore, Config, Serve)]
#[server(config = MyConfig)]
struct MyServer;

// Opt out of the config subcommand within blessed
#[derive(Server)]
#[server(config = MyConfig, config_cmd = false)]
struct MyServer;

// Custom subcommand name
#[derive(Server)]
#[server(config = MyConfig, config_cmd = "settings")]
struct MyServer;
```

## `#[param]` on Config Fields

Config fields use the same `#[param]` attribute as method parameters, extended with config-specific keys:

| Attribute | Effect |
|-----------|--------|
| `env = "VAR"` | Map to environment variable |
| `default = ...` | Compiled-in default value |
| `help = "..."` | Description shown in `config show` and `--help` |
| `file_key = "a.b.c"` | Override the key path in the config file (default: field name) |

Additional attributes may be added as needs become clearer — the field-level annotation surface is intentionally minimal for now.

## Delegation

Config file parsing delegates to [figment](https://crates.io/crates/figment), following the same delegation pattern as HTTP (axum), CLI (clap), etc. Server-less owns the derive interface and the generated subcommand; figment owns the merging and file parsing logic.

## Prior Art

The `config` subcommand design is directly inspired by `normalize config`, which provides the same four sub-commands (`show`, `schema`, `validate`, `set`) with the schema-annotated display and dotted-path section navigation. The normalize implementation is itself built on server-less's `#[cli]` macro.

## Open Questions

- **`--config <path>` flag**: Configurable via `#[config(config_flag = true/false)]`. Whether to default to true or false depends on whether a top-level `--config` override is generally considered desirable in server-less-generated CLIs. Lean toward true.

- **Nested config structs**: `#[derive(Config)]` on the nested struct is required *iff* you want schema/discoverability on that sub-struct (e.g. for `config show` to render its fields and `config schema` to include its shape). The parent derive does not force it — it's opt-in per sub-struct.

- **`config set` file format**: Configurable via `#[config(format = "toml")]`. Most tools only support a single format; TOML is the default (it's the only common format that roundtrips without losing comments).

- **Default config file path**: `{name}.toml` falling back to `config.toml`, where `name` is the application name. This requires a top-level name concept — see below.

- **Top-level application name and description**: Resolved — see [Application Metadata](./app-metadata.md). The config file default path uses `name` from `#[app]`, falling back to `config.toml` if no name is set.
