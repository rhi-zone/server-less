# normalize `--schema` flag wiring

## Why the --schema pre-clap hack exists

### Provenance

The `--schema` flag was introduced in commit `d19654c9` (2026-01-11) in the
then-named `moss` repo as `feat: add --schema flag for Nursery integration`.
A follow-up commit `1f24fd15` (`feat(cli): implement --schema flag for Nursery
integration`) replaced the stub `NurseryConfig` struct with
`schemars::schema_for!(MossConfig)` (the real config type), and wrapped the
output in a JSON envelope with `config_path` and `format` fields.

The "before clap parsing" comment was present from day one in `d19654c9`. The
commit message reads:

> Implements the Nursery tool integration convention. When invoked with
> `--schema`, moss prints a JSON Schema describing its configuration for use
> with Nursery's declarative pipeline system.

There are no linked issues or inline TODOs explaining the bypass, but the code
structure makes the reason unambiguous (see below).

### The wall: why it had to be pre-clap

At the time `--schema` was introduced, `NormalizeService` (then `MossService`)
was wired to clap with:

```rust
#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,   // required field, no Option<>
    ...
}
```

A bare `normalize --schema` (no subcommand) fails clap parsing with a "required
argument missing" error before any application code runs. Clap never reaches
dispatch. The only way to handle `--schema` without a subcommand was to
intercept `std::env::args()` before calling `Cli::parse()`.

This is not a deficiency in clap itself — it is the direct consequence of
`Commands` being a required non-optional field. Adding `--schema` as a clap
`Arg` on `Cli` wouldn't help either, because clap still requires the subcommand
regardless. Making it work purely through clap would have required either
restructuring the entire subcommand as `Option<Commands>` (invasive) or adding
a `Schema` variant to `Commands` (wrong layer — the schema isn't a command).

The pre-clap hack is thus the minimal-invasive correct choice given the existing
CLI shape.

### Was server-less's #[cli] / #[program] a factor?

Partially. At the time `--schema` was introduced (2026-01-11), normalize was
still on the raw clap `Cli`/`Commands` structure — the server-less `#[cli]`
migration had not yet begun (it started with `e7cd91b1`, 2026-01-30). So at
introduction time, server-less was not in the picture at all.

After the migration, `NormalizeService` uses `#[cli]` but does NOT pass
`config = NormalizeConfig` to the macro, so server-less never generates a
`config schema` subcommand for normalize. Server-less's `config schema`
subcommand (added in `bfd6897`, 2026-03-09) would have been the sanctioned path
for this, but normalize's `#[cli]` attribute was never wired up to it.

### Is there a sanctioned server-less path today?

Yes. `NormalizeConfig` already derives `server_less::Config` (confirmed in
`crates/normalize/src/config.rs` line 117). Server-less's `#[program]` or
`#[server]` macro accepts `config = NormalizeConfig` and, when present,
generates a `normalize config schema` subcommand that calls
`_config_schema()` — which emits a JSON Schema from the `ConfigLoad` field
metadata.

The existing `--schema` contract (Nursery reads `{ config_path, format, schema
}`) differs from server-less's `config schema` output (which emits a raw JSON
Schema object, not the envelope), but that difference is small and fixable.

### Verdict

This is a **removable workaround**, not a genuine server-less gap.

- The original bypass was inevitable given the required-subcommand CLI shape
  and the pre-server-less codebase state (2026-01-11).
- Server-less grew the `config = T` / `config schema` surface on 2026-03-09,
  after normalize's server-less migration was complete.
- `NormalizeConfig` already derives `server_less::Config`, so wiring
  `config = NormalizeConfig` into the `#[cli]` macro on `NormalizeService` is
  the remaining step.
- The `--schema` top-level flag would then need to be either retired (redirect
  Nursery to `normalize config schema`) or kept as a thin shim that calls
  `normalize config schema` internally — but the pre-clap intercept itself
  could be removed once Nursery is updated to use the subcommand form.

## Can a root/intermediate node be both container and leaf?

**Crux question:** Does the `#[cli]` model allow a node that has subcommands
AND has its own callable return type / output schema — i.e., a node that is
simultaneously a container (routes to children) and a leaf (produces output
when invoked directly)?

### What is built (macro source evidence)

**The root command** (`CliSubcommand::cli_dispatch` on the annotated `impl`)
dispatches via a `match matches.subcommand()` on all pattern arms. The fallthrough
`_ =>` arm is:

```rust
// crates/server-less-macros/src/cli.rs lines 986-989, 1004-1007
_ => {
    Self::cli_command().print_help()?;
    Ok(())
}
```

No subcommand given → print help, return `Ok(())`. The root produces no data
output, has no return type of its own, and owns no output schema. It is a pure
container/dispatcher.

**Mount point nodes** (methods returning `&T`) are likewise pure containers.
Their dispatch arm calls `cli_dispatch` on the child and returns:

```rust
// lines 1801-1806
Some((#subcommand_name, sub_matches)) => {
    let __delegate = self.#method_name();
    <#inner_ty as ::server_less::CliSubcommand>::cli_dispatch(__delegate, sub_matches)
}
```

The mount method itself is never called for its return value — the macro calls
it only to obtain the `&T` delegate, then hands off immediately. There is no
output logic, no `--output-schema` check, and no display code on the mount arm.
The `generate_static_mount_subcommand` function (lines 1094-1121) similarly
never attaches schema flags or output logic to the mount-level command.

**`--output-schema` is only generated inside `generate_leaf_match_arm`** (lines
1383-1417), and `generate_leaf_match_arm` is only called for leaf methods
(methods that are NOT mount points). Mount point arms are generated by
`generate_static_mount_arm` / `generate_slug_mount_arm`, which contain no
schema logic whatsoever.

**`#[cli(default)]`** (lines 299-319, 636-700) is the only mechanism that
allows a method to run when no subcommand is given. It works by generating a
`None =>` arm in `cli_dispatch` that runs the marked leaf method using the
parent command's `ArgMatches`. Critically, this method is a **leaf** (it has
its own return value, output code, and `--output-schema` handling). It is NOT
a mount-point method. With `#[cli(default)]`, the root command becomes
"invocable" in the sense that running the binary bare invokes that one leaf
method — but only one method can be marked default, and the root node does not
independently possess a return type or output schema; the schema and output
belong to the default leaf method, accessed via the flattened `None` arm.

### What the design docs say

`docs/design/mount-points.md` defines mount points exclusively as methods
returning `&T` where `T: CliSubcommand`. It describes no case where a mount
point also has its own output:

> "Methods returning `&T` become subcommand groups (mount points)"
> "`app health` calls the leaf. `app users <subcommand>` descends into
> `UserService`'s command tree."

The two categories (leaf vs mount) are presented as exclusive. There is no
"leaf-mount hybrid" concept anywhere in the doc or in the implemented code.

`docs/design/cli-output-formatting.md` describes `--output-schema` as
belonging to "the subcommand's return type" — referring implicitly to leaf
subcommands. Mount-level commands are not discussed as having output schemas.

`docs/design/impl-first.md` confirms the structural partition:

> "Each `pub` method with `&self` becomes a subcommand" — leaf
> "Methods returning `&T` become subcommand groups" — mount

### Verdict

**Container-nodes-that-are-also-leaves do not exist in the current model,
built or designed.**

1. **Root command:** always a pure dispatcher. Bare invocation prints help
   (or, with `#[cli(default)]`, runs one designated leaf method). The root
   does not have its own return type or output schema. `#[cli(default)]` adds
   a callable leaf reachable at the root position, but it is the leaf's schema
   that is consulted, not a separate root-level schema.

2. **Mount point (intermediate) nodes:** pure containers. The mount arm
   delegates to `cli_dispatch` on the child without any output logic or schema
   checks. There is no mechanism to attach an output schema to a mount-point
   method, and the design docs contain no such concept.

3. **`--output-schema` is leaf-only:** it is generated exclusively inside
   `generate_leaf_match_arm`, never in mount arm generators.

### Implication for --output-schema overloading

Overloading `--output-schema` by position — root invocation → aggregate schema
for the whole tree, leaf invocation → that leaf's return type schema — will NOT
collide with any existing "root/intermediate node that has its own output
schema," because no such node currently exists. The only collision risk is with
`#[cli(default)]`: if the default method's `--output-schema` is the designated
root-level schema, overloading must decide whether `--output-schema` with no
subcommand means "the default method's schema" (current behavior) or "aggregate
tree schema" (proposed overload). That is the one ambiguity to resolve.
