# `#[cli]` Method Attributes

Reference for all method-level `#[cli(...)]` attributes accepted by the `#[cli]` proc macro.

## Overview

When `#[cli]` is applied to an impl block, each `pub` method with `&self` becomes a
subcommand by default. Method-level attributes let you control naming, visibility,
default actions, and output formatting.

```rust
#[cli(name = "myapp")]
impl MyApp {
    #[cli(default, display_with = "show")]
    pub fn status(&self, verbose: bool) -> Status { ... }

    #[cli(hidden)]
    pub fn internal_debug(&self) -> Debug { ... }

    #[cli(name = "rm")]
    pub fn remove(&self, id: String) -> Result<(), Error> { ... }
}
```

---

## Attribute Reference

### `#[cli(skip)]`

Exclude the method from CLI generation entirely. No subcommand is registered; no
dispatch arm is generated. Equivalent to `#[cli(helper)]`.

**Use when:** the method contains implementation logic that should not be a subcommand
(e.g., a private helper that happens to be `pub` for testing).

```rust
#[cli(skip)]
pub fn internal_helper(&self) -> String { ... }
```

---

### `#[cli(helper)]`

Self-documenting alias for `#[cli(skip)]`. Prefer `helper` over `skip` for display
formatters, shared logic, and methods that exist to serve other subcommands.

```rust
#[cli(helper)]
pub fn format_output(&self, data: &Data) -> String { ... }
```

---

### `#[cli(hidden)]`

Register the method as a named subcommand but hide it from `--help` output. The
subcommand still works when invoked explicitly; it just isn't listed.

**Use when:** you want a method accessible as an explicit subcommand but not
advertised — e.g., an escape hatch, a migration shim, or a default action that happens
to share a name with its parent service.

```rust
#[cli(hidden)]
pub fn legacy_mode(&self) -> Output { ... }
// works: myapp legacy-mode
// hidden from: myapp --help
```

**Default + hidden:** if a method is both the default action and has a confusing name
(e.g., the method name duplicates the service name), combine both:

```rust
#[cli(default, hidden, display_with = "show")]
pub fn context(&self) -> Output { ... }
// works: myapp           (default action)
// works: myapp context   (explicit, undocumented)
// hidden from: myapp --help
```

---

### `#[cli(default)]`

Make this method the default action when the user invokes the service with no
subcommand. The method IS also registered as a named subcommand (unlike `hidden`,
which is orthogonal).

Its parameters are hoisted to the parent command's argument list so they work as
top-level flags.

**Use when:** a service has one primary action and several supporting subcommands.
The primary action runs when the user types the service name alone; the others are
explicit.

```rust
#[cli(name = "analyze")]
impl AnalyzeService {
    /// Run health analysis (also the default)
    #[cli(default)]
    pub fn health(&self, target: Option<String>) -> Report { ... }

    /// Run all analysis passes
    pub fn all(&self) -> AllReport { ... }
}
// normalize analyze            → runs health (default)
// normalize analyze health     → runs health (explicit)
// normalize analyze all        → runs all
```

**Only one method per impl block may be marked `default`.** A compile error is emitted
if multiple methods carry this attribute.

**`default` + `hidden`:** if you want the default action to NOT appear in the
subcommand list (e.g., its name would be confusing), combine both attributes:

```rust
#[cli(default, hidden, display_with = "show")]
pub fn query(&self) -> Output { ... }
```

---

### `#[cli(name = "...")]`

Override the subcommand name. By default, method names are converted to kebab-case
(`create_user` → `create-user`). Use `name` to provide a different name.

Works on both leaf methods and mount points.

```rust
#[cli(name = "rm")]
pub fn remove(&self, id: String) -> Result<(), Error> { ... }
// myapp rm <id>   (not: myapp remove <id>)
```

---

### `#[cli(alias = "...")]`

Add one or more **hidden** aliases for the subcommand. The command is invocable under
each alias, but the alias does **not** appear in `--help` (it uses clap's `.alias(...)`,
which is hidden by default — unlike `.visible_alias(...)`).

Repeatable, or use the list form:

```rust
#[cli(name = "architecture", alias = "arch", alias = "analyze-architecture")]
pub fn architecture(&self) -> Report { ... }

// equivalently:
#[cli(name = "architecture", aliases = ["arch", "analyze-architecture"])]
pub fn architecture(&self) -> Report { ... }

// myapp architecture           (shown in --help)
// myapp arch                    (works, hidden)
// myapp analyze-architecture    (works, hidden)
```

Works on both leaf methods and mount points. The primary use case is **migration
scaffolding**: when a verb is renamed or moved, keep its old command path as a hidden
alias for one release so existing invocations keep working without advertising the
deprecated spelling.

---

### `#[cli(display_with = "fn_name")]`

Use a custom method for text output. Without this attribute, output uses the
`Display` impl (or the macro's built-in handling for `Vec<T>`, `Option<T>`, etc.).

The named function must be a method on the same type (any impl block, not just the
`#[cli]` block). It is called as `self.fn_name(&return_value)` and must return
`String`.

`--json` and `--jq` always bypass `display_with` and use `serde_json` serialization.

```rust
fn show_report(&self, r: &Report) -> String {
    if self.pretty.get() { r.format_pretty() } else { r.format_text() }
}

#[cli(display_with = "show_report")]
pub fn health(&self) -> Report { ... }
```

---

## Combining Attributes

Attributes are independent flags on the same `#[cli(...)]` annotation:

```rust
// default action, hidden from --help, custom display
#[cli(default, hidden, display_with = "show")]
pub fn context(&self) -> Output { ... }

// renamed + custom display
#[cli(name = "rm", display_with = "show_removal")]
pub fn remove(&self, id: String) -> Result<(), Error> { ... }
```

---

## Patterns

### Service with a primary action and subcommands

The default action runs when no subcommand is given. It also appears as an explicit
subcommand so `--help` can describe it:

```rust
#[cli(name = "analyze")]
impl AnalyzeService {
    /// Default: run health analysis
    #[cli(default, display_with = "display")]
    pub fn health(&self, target: Option<String>) -> HealthReport { ... }

    pub fn all(&self) -> AllReport { ... }
    pub fn summary(&self) -> SummaryReport { ... }
}
```

```
$ analyze             # runs health (default)
$ analyze health      # runs health (explicit)
$ analyze all         # runs all
```

### Hiding a duplicate default action

When the default method's name would duplicate the parent service name in `--help`
(e.g., a `context` method on `ContextService` named `context`), mark it `hidden` to
suppress it from the subcommand list while keeping it accessible:

```rust
#[cli(name = "context")]
impl ContextService {
    #[cli(default, hidden, display_with = "show")]
    pub async fn context(&self, ...) -> Result<ContextReport, Error> { ... }

    pub async fn migrate(&self, apply: bool) -> MigrateReport { ... }
}
```

```
$ context              # runs context (default, parameters hoisted to top level)
$ context migrate      # runs migrate
$ context --help       # shows: migrate, help  (context omitted — avoids "context context")
```

### Internal helpers

Methods that serve other subcommands but shouldn't be callable directly:

```rust
#[cli(helper)]
pub fn format_as_table(&self, data: &[Row]) -> String { ... }
```
