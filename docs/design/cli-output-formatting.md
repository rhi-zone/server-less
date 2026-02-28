# CLI Output Formatting

## Decision

CLI output defaults to `Display`, with `--json`/`--jsonl`/`--jq` opting into machine-readable output. The macro handles common standard library types (Result, Option, Vec, HashMap, BTreeMap) at expansion time. A `display_with` attribute provides an escape hatch for custom formatting.

## Context

CLI tools conventionally output human-readable text by default (`ls`, `git status`) and structured JSON only when asked (`--json`). server-less initially routed all output through `serde_json::to_value()` into pretty-printed JSON. This should be flipped.

The question is: what trait governs the human-readable default?

## Alternatives Considered

### Display for everything

Use `println!("{}", value)` unconditionally. Simple, but `Vec<T>`, `HashMap<K,V>`, and other std collections don't implement `Display`. Since these are extremely common CLI return types (every "list" command returns a Vec), this creates immediate friction.

### Custom trait (CliDisplay)

Define a `CliDisplay` trait with blanket impls for `Vec<T: CliDisplay>`, `HashMap<K, V>`, etc. This works around orphan rules for *us* — but downstream consumers can't override `impl CliDisplay for Vec<TheirType>` because both `CliDisplay` and `Vec` are foreign to them. We'd be deciding collection formatting and they're stuck with it.

### Serde-based renderer

Walk `serde_json::Value` and format human-readably (strings without quotes, arrays one-per-line, etc.). No new traits needed, works for any `Serialize` type. But this bypasses Display entirely — types that *do* have carefully written Display impls (error types, domain objects) would be ignored in favor of a generic JSON-based rendering.

### Display + macro-handled types + display_with (chosen)

Display is the default. The macro handles a finite set of std types at expansion time. `display_with` overrides formatting per-method.

## Design

### Default output by return type

The macro already parses return types. It generates different output code depending on what it finds:

| Return type | Default output |
|---|---|
| `()` | `println!("Done")` |
| `Result<(), E>` | Ok: `println!("Done")`, Err: Display the error |
| `Result<T, E>` | Ok: display T, Err: display E |
| `Option<T>` | Some: display T, None: "Not found" + exit 1 (with `--json`: `null`) |
| `Vec<T>` | One item per line, each via Display |
| `impl Iterator<Item=T>` | Streams one JSON line per item (same as `--jsonl`) |
| `HashMap<K,V>`, `BTreeMap<K,V>` | `key: value` per line, each via Display |
| anything else | `println!("{}", value)` (Display) |

Errors use `Display` (not `Debug`) since well-designed error types (miette, anyhow, std::io::Error) implement Display with human-readable messages.

### JSON output (opt-in)

When `--json`, `--jsonl`, or `--jq` is passed, all paths serialize via `serde_json::to_value()` and go through `cli_format_output`. This requires `Serialize` on the return type, which is already required today.

**Exception: `impl Iterator<Item=T>`** — iterators are streamed to avoid unbounded memory use:

| Flag | Behavior |
|---|---|
| default (no flags) | Stream JSONL: one `serde_json::to_string` line per item |
| `--jsonl` | Same as default |
| `--json` | Collect all items into a `Vec`, serialize as JSON array. **Unsafe for infinite iterators** — will exhaust memory. |
| `--jq` | Collect all items, serialize as array, apply jq filter. Same caveat. |

For truly infinite sequences use `impl Stream<Item=T>` instead, which is fully async and backpressure-aware.

`--jq` filtering uses the [jaq](https://github.com/01mf02/jaq) library (`jaq-core`, `jaq-std`, `jaq-json`) — no external `jq` binary needed. Consistent behavior across platforms, no subprocess overhead.

### Schema introspection

Two global flags allow programmatic discovery of command shapes:

- `--input-schema` — prints the JSON Schema of the subcommand's input parameters and exits
- `--output-schema` — prints the JSON Schema of the return type and exits

When the `jsonschema` feature is enabled and the return type implements `schemars::JsonSchema`, `--output-schema` uses schemars for accurate schema generation. Otherwise it falls back to a heuristic based on the parsed return type.

### display_with escape hatch

For methods where the default doesn't fit:

```rust
#[cli(display_with = "format_items")]
pub fn list_items(&self) -> Vec<Item> { ... }

// Method on &self — has access to whatever context the user stores
fn format_items(&self, items: &Vec<Item>) -> String {
    items.iter().map(|i| format!("  - {}", i.name)).collect::<Vec<_>>().join("\n")
}
```

The function is a method on the same struct, so the user's struct *is* the context — no server-less-owned context type needed. This follows serde's `#[serde(serialize_with = "...")]` pattern — progressive disclosure. You discover it when you need it, not before.

When `--json`, `--jsonl`, or `--jq` is passed alongside `display_with`, the JSON path takes precedence — the value is serialized via serde, not the custom formatter. This ensures machine-readable output is always structurally consistent regardless of display customization.

### `--params-json`

All subcommands accept `--params-json <JSON>`, which provides every parameter as a single JSON object instead of individual flags:

```bash
# These are equivalent:
myapp create-user --name "Alice" --email "alice@example.com"
myapp create-user --params-json '{"name": "Alice", "email": "alice@example.com"}'
```

Keys are the snake_case parameter names (matching the Rust function signature, not the kebab-case CLI flag names). Values are strings that get `.parse()`d into the target type, matching how individual CLI flags work.

`--params-json` composes with output formatting flags — you can combine it with `--json`, `--jq`, etc. This is particularly useful for scripting, where constructing a JSON object is often easier than escaping shell arguments.

### `#[cli(global = [...])]`

Declares global boolean flags that propagate to all subcommands:

```rust
#[cli(name = "myapp", global = [verbose, debug])]
impl MyService {
    pub fn list_items(&self) -> Vec<Item> { ... }
    pub fn get_item(&self, item_id: String) -> Option<Item> { ... }
}
```

This adds `--verbose` and `--debug` as `SetTrue` flags on *every* subcommand. They're automatically filtered from each subcommand's own argument list (so a method parameter named `verbose` won't conflict). Global flags are extracted from the parent `ArgMatches` before dispatching to the subcommand.

Optional help text can be provided per flag:

```rust
#[cli(name = "myapp", global = [
    pretty = "Format output as pretty-printed JSON",
    compact = "Format output as compact JSON",
])]
```

Without `= "..."`, the flag appears in `--help` with no description.

### `#[cli(defaults = "fn_name")]`

Provides a fallback for required parameters that aren't supplied on the command line:

```rust
#[cli(defaults = "get_defaults")]
impl MyService {
    pub fn connect(&self, host: String, port: u16) -> String { ... }

    fn get_defaults(&self, key: &str) -> Option<String> {
        match key {
            "host" => Some("localhost".to_string()),
            "port" => Some("8080".to_string()),
            _ => None,
        }
    }
}
```

**Signature:** `fn(&self, key: &str) -> Option<String>` — the `key` is the kebab-case parameter name (e.g., `"host"`, `"port"`). Return `Some(value)` to provide a default, `None` to leave it required.

The returned string is `.parse()`d into the target type, so `"8080"` becomes `8080u16`. This happens after CLI parsing but before method dispatch — if the user provides an explicit value, the defaults function is never called for that parameter.

This differs from `#[param(default = ...)]`: `#[param(default)]` is a compile-time constant baked into the generated code (and affects HTTP/OpenAPI too), while `#[cli(defaults)]` is a runtime function with access to `&self` — it can read config files, environment variables, or any other state.

### Why this works

- **Display covers most types naturally.** Primitives, String, error types, and user structs that are meant for human consumption already impl Display.
- **Collections are a finite set.** Vec, HashMap, BTreeMap — not an ever-growing list. The macro handles them once.
- **No orphan rule problems.** No custom traits needed. Users impl Display (a std trait) for their own types.
- **Consumers keep control.** `display_with` lets them format anything however they want, with access to output context.
- **Progressive disclosure.** Level 1: return a Display type, it just works. Level 2: return a Vec, it just works. Level 3: need custom formatting, use `display_with`.
