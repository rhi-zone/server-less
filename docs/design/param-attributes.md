# `#[param]` Attribute Design

How `#[param(...)]` works, why it's cross-protocol, and how it relates to clap's `#[arg]`.

## `#[param]` Is Cross-Protocol

`#[param]` lives on function parameters, and the same parameter may be projected into
multiple protocols by different derives on the same impl block. This means `#[param]`
attributes need to make sense across HTTP, CLI, MCP, JSON-RPC, etc.

```rust
#[http] #[cli] #[mcp]
impl UserService {
    pub fn get_user(
        &self,
        #[param(help = "User to look up")]
        user_id: UserId,
    ) -> Option<User> { ... }
}
```

The `help` text appears in CLI `--help` output, as the MCP tool parameter description,
and potentially in OpenAPI parameter descriptions. One annotation, multiple protocols.

Some attributes are protocol-specific:

| Attribute | Protocol | Effect |
|-----------|----------|--------|
| `query`, `path`, `body`, `header` | HTTP only | Override parameter location |
| `short = 'x'` | CLI only | Add a short flag |
| `positional` | CLI only | Make the argument positional |
| `name = "..."` | HTTP, OpenAPI | Override the wire name (CLI ignores this; use `#[cli(name)]` for CLI) |
| `help = "..."` | All | Description text |
| `default = ...` | HTTP, OpenAPI | Default value (CLI uses `#[cli(defaults)]` separately) |

Protocol-specific attributes on non-matching derives are silently ignored — `short`
has no meaning in HTTP and doesn't affect HTTP code generation.

## Why Not Mirror clap's `#[arg]`?

clap's `#[arg]` API is rich: `env`, `value_parser`, `alias`, `hide`, `num_args`,
`value_delimiter`, `conflicts_with`, and many more. Committing to full parity would
mean reimplementing clap_derive, with all the maintenance burden that implies.

We don't claim to be clap. We claim to be a thin projection layer that calls clap on
your behalf. Users who need full clap control have the escape hatch: skip `#[cli]` on
that impl block and write the clap command manually.

That said, for attributes we *do* support, we prefer clap's naming when it doesn't
conflict with cross-protocol concerns:

- `short = 'v'` — matches clap
- `help = "..."` — matches clap

Where we diverge:

- `name = "..."` — clap uses `long = "..."` for the flag name and `value_name = "..."`
  for the metavar. We use `name` because it's the wire name across *all* protocols, not
  just the CLI long flag.
- `positional` — clap uses `index = N` for explicit positional ordering. We use a
  boolean flag instead (see below).

## Positional Arguments

`#[param(positional)]` marks a parameter as a positional CLI argument rather than a
`--flag` argument.

```rust
pub fn search(
    &self,
    #[param(positional)] pattern: String,
    path: Option<String>,
) -> Vec<Match>
// → myapp search <pattern> [--path <path>]
```

**Why `positional` instead of `index = N`?**

clap requires `index = N` because it can't infer ordering — you might add `#[arg]`
annotations out of order, or the struct fields might be reordered. Neither problem
applies here: function parameters have a fixed, meaningful declaration order. The
*N*th positional argument in the function signature is naturally the *N*th positional
argument in the CLI.

Assigning explicit indices would be redundant at best and a source of bugs at worst
(what happens when you reorder parameters and forget to update the indices?). Instead,
positional index is inferred from declaration order:

```rust
pub fn copy(
    &self,
    #[param(positional)] src: String,   // → index 1
    #[param(positional)] dst: String,   // → index 2
) -> Result<(), IoError>
// → myapp copy <src> <dst>
```

The function signature is the source of truth. No redundant annotations needed.

> **Note:** Multiple positional arguments on a single command (like the `copy` example above) are not yet implemented — the positional index is currently hardcoded to 1. The design is settled, but the codegen needs updating to assign sequential indices. For now, only one positional argument per command works correctly.

## The `is_id` Heuristic

Parameters named `id`, `user_id`, `post_id`, etc. (ending in `_id` or named `id`)
are automatically treated as positional. This covers the common case without requiring
any annotation:

```rust
pub fn get_user(&self, user_id: String) -> Option<User>
// → myapp get-user <user-id>   (no annotation needed)
```

`#[param(positional)]` is the explicit version, preferred when the heuristic doesn't
apply or when you want to be self-documenting.

## See Also

- [CLI Output Formatting](cli-output-formatting.md) — `display_with`, `--json`/`--jq`, `defaults`
- [Route & Response Attributes](route-response-attrs.md) — `#[route]` and `#[response]` for HTTP overrides
