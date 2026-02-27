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
| `Option<T>` | Some: display T, None: "Not found" + exit 1 |
| `Vec<T>` | One item per line, each via Display |
| `HashMap<K,V>`, `BTreeMap<K,V>` | `key: value` per line, each via Display |
| anything else | `println!("{}", value)` (Display) |

Errors use `Display` (not `Debug`) since well-designed error types (miette, anyhow, std::io::Error) implement Display with human-readable messages.

### JSON output (opt-in)

When `--json`, `--jsonl`, or `--jq` is passed, all paths serialize via `serde_json::to_value()` and go through `cli_format_output`. This requires `Serialize` on the return type, which is already required today.

### display_with escape hatch

For methods where the default doesn't fit:

```rust
#[cli(display_with = "format_items")]
pub fn list_items(&self) -> Vec<Item> { ... }
```

The function receives the value and an output context:

```rust
fn format_items(items: &Vec<Item>, ctx: &OutputContext) -> String {
    // ctx carries compact, color, etc.
}
```

This follows serde's `#[serde(serialize_with = "...")]` pattern — progressive disclosure. You discover it when you need it, not before.

### Why this works

- **Display covers most types naturally.** Primitives, String, error types, and user structs that are meant for human consumption already impl Display.
- **Collections are a finite set.** Vec, HashMap, BTreeMap — not an ever-growing list. The macro handles them once.
- **No orphan rule problems.** No custom traits needed. Users impl Display (a std trait) for their own types.
- **Consumers keep control.** `display_with` lets them format anything however they want, with access to output context.
- **Progressive disclosure.** Level 1: return a Display type, it just works. Level 2: return a Vec, it just works. Level 3: need custom formatting, use `display_with`.
