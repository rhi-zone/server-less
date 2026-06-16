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
