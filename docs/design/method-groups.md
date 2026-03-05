# Method Groups

Cross-protocol grouping for methods. Groups organize methods into named sections — CLI help headings, OpenAPI tags, markdown sections, JSON-RPC method categories.

## Motivation

Large service impls accumulate dozens of methods. Without structure, `--help` becomes a wall of text and API docs become a flat list. Users need semantic grouping: "these methods are about code quality, those are about module structure."

Mount points (`&T` return types) provide *structural* grouping — a whole sub-service with its own type. Groups provide *annotated* grouping — lightweight categories within a single impl, without requiring separate types.

## Design

Two tiers, following progressive disclosure:

### Tier 1: Inline strings (simple)

```rust
#[server]
impl AnalyzeService {
    #[server(group = "Code quality")]
    pub fn complexity(&self, path: String) -> ComplexityReport { ... }

    #[server(group = "Code quality")]
    pub fn length(&self, path: String) -> LengthReport { ... }

    #[server(group = "Module structure")]
    pub fn density(&self, path: String) -> DensityReport { ... }

    // Ungrouped — appears before any group section
    pub fn summary(&self) -> SummaryReport { ... }
}
```

- The string *is* the display name.
- Group ordering in output = first-seen order in the impl block.
- Typos produce an extra group (visible immediately in `--help` / docs).
- Zero ceremony — one attribute, one string.

### Tier 2: Registry with IDs (strict)

```rust
#[server]
#[server(groups(
    code = "Code quality",
    modules = "Module structure",
    repo = "Repository",
    graph = "Graph analysis",
))]
impl AnalyzeService {
    #[server(group = "code")]
    pub fn complexity(&self, path: String) -> ComplexityReport { ... }

    #[server(group = "modules")]
    pub fn density(&self, path: String) -> DensityReport { ... }

    // Ungrouped
    pub fn summary(&self) -> SummaryReport { ... }
}
```

- `groups(...)` on the impl block declares the registry: `id = "Display Name"`.
- `group = "id"` on a method must match a declared ID — compile error otherwise.
- Group ordering in output = declaration order in `groups(...)`.
- IDs are internal linkage — never shown to users, only the display name appears.

### Disambiguation

When `groups(...)` is declared on the impl block, `group = "x"` resolves against registry IDs. When no registry exists, `group = "x"` is a literal display name (Tier 1).

This means the two tiers coexist without ambiguity: the presence of `groups(...)` switches the interpretation.

## Cross-Protocol Projection

Groups are declared once via `#[server(group = "...")]` and projected to each protocol:

| Protocol | Projection |
|----------|-----------|
| CLI | Clap subcommand `help_heading` — methods appear under their group's display name |
| OpenAPI | `tags` on each operation — group display name becomes the tag |
| Markdown | Section headers (`## Group Name`) with methods listed under their group |
| JSON-RPC | No visible effect (flat namespace), but available in introspection metadata |
| MCP | Tool annotations — group name in tool description metadata |
| WebSocket | No visible effect, available in introspection |

### Interaction with `#[route(tags = "...")]`

`#[route(tags)]` remains for HTTP-specific OpenAPI tags. When both `group` and `route(tags)` are present, the group display name is *prepended* to the explicit tags list. This lets you have cross-protocol grouping plus HTTP-specific extra tags:

```rust
#[server(group = "admin")]
#[route(tags = "legacy,deprecated")]
pub fn old_endpoint(&self) -> String { ... }
// OpenAPI tags: ["Admin", "legacy", "deprecated"]
```

## CLI Help Output

```
Usage: analyze <COMMAND>

Commands:
  summary     Codebase health summary
  all         Run all analyses

Code quality:
  complexity  Rank functions by cyclomatic complexity
  length      Rank functions by line count

Module structure:
  density     Information density per module
  coupling    Cross-module dependency analysis

Repository:
  churn       File change frequency
  hotspots    High-churn + high-complexity intersections
```

Ungrouped methods appear first under the standard `Commands:` heading. Grouped methods appear in sections ordered by group declaration (Tier 2) or first-seen (Tier 1).

## Parse Layer

`server_less_parse::MethodInfo` gains:

```rust
pub struct MethodInfo {
    // ... existing fields ...
    pub group: Option<String>,  // raw value from #[server(group = "...")]
}
```

A new type for the impl-level registry:

```rust
pub struct GroupRegistry {
    /// Ordered list of (id, display_name) pairs.
    pub groups: Vec<(String, String)>,
}
```

`extract_methods()` doesn't change — it already receives `&ItemImpl` and can access impl-level attrs. Group parsing is a new function:

```rust
pub fn extract_groups(impl_block: &ItemImpl) -> syn::Result<Option<GroupRegistry>>
```

Each protocol's expander calls `extract_groups()` and resolves method group strings against the registry (if present) or uses them as literal display names.

### Validation

When a `GroupRegistry` is present:
- `group = "x"` where `x` is not a declared ID → compile error with span pointing at the attribute
- Declared groups with zero methods → warning (unused group)

When no registry:
- Any string is accepted — no validation needed

## Implementation Surface

1. **`server-less-parse`** — `extract_groups()` parses `groups(...)` from impl attrs. `MethodInfo::parse()` reads `#[server(group = "...")]` into the new field.
2. **`server-less-macros/cli.rs`** — `generate_cli()` partitions subcommands by group, emits `help_heading` on each clap `Command`.
3. **`server-less-macros/openapi.rs`** — prepends group display name to operation tags.
4. **`server-less-macros/markdown.rs`** — emits group sections.
5. **`server-less-core`** — `MethodInfo` (runtime) gains `group: Option<String>` for introspection.

## Alternatives Considered

**Registry only (no Tier 1).** Rejected — too much ceremony for simple cases. A service with 3 groups doesn't need a registry; inline strings work fine.

**Inline only (no Tier 2).** Rejected — typos in group names silently create extra groups. Large impls benefit from compile-time validation and explicit ordering.

**Per-protocol group attrs (`#[cli(group)]`, `#[route(group)]`).** Rejected — groups are a semantic concept that should project consistently. Per-protocol overrides could be added later if needed, but the common case is one grouping across all protocols.

**Clap `ArgGroup`.** Not applicable — `ArgGroup` is for mutually exclusive *arguments*, not for organizing *subcommands* into sections. Clap uses `help_heading` for subcommand sections.
