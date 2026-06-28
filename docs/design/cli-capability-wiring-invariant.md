# CLI capability-wiring invariant — closing the "declared-but-silently-inert" class

**Date:** 2026-06-28
**Status:** IMPLEMENTED (v0.6.0) for 3a (`global`/`CliGlobals`), 3c (`#[param(name)]`,
`#[param(default)]`). 3b (`display_with` opaque body) and 3d (`defaults` on all-optional
impl) remain open — see §8 and TODO. The `compile_error!` interim guard of §3a was
**subsumed** by the `CliGlobals` bound (see "Reconciliation" below).
**Scope:** `crates/server-less-macros/src/cli.rs`, `crates/server-less-parse/src/lib.rs`,
`crates/server-less-core/src/lib.rs` (the `CliGlobals` trait)

> **Implementation note (2026-06-29).** Landed the structural centerpiece (3a) and both
> cheap codegen fixes (3c). Key code: `CliGlobals` trait in
> `crates/server-less-core/src/lib.rs`; per-leaf delivery `<Self as CliGlobals>::set_global_flag(...)`
> + leaf-less-edge bound assertion in `crates/server-less-macros/src/cli.rs`
> (`generate_leaf_match_arm` / `generate_cli`); `cli_param_name` honors `wire_name` at every
> flag-name + extraction site; `clap_default_value` + `default_clause` in `generate_arg`.
> Compile-fail proof: `crates/server-less/tests/fixtures/cli_global_without_sink.rs`
> (`.stderr` = `E0277: the trait bound MyApp: CliGlobals is not satisfied`). Compile-pass:
> `test_param_name_renames_flag`, `test_param_default_sets_clap_default`,
> `test_cli_globals_delivers_flag_values` in `tests/cli_tests.rs`.
>
> **Reconciliation of the §3a interim `compile_error!` (design-D param-presence variant):**
> it is now **subsumed and dropped**, not implemented. Its purpose was to require a method
> to re-declare a matching param so a global flag's value would reach the body. With
> `CliGlobals`, the global is delivered to the *sink*, not to method params — so requiring a
> matching param would be backwards. The `Self: CliGlobals` bound is strictly stronger
> (it enforces the existence of the *termination*, which the param-presence check never did)
> and is enforced unconditionally (per-leaf delivery + a leaf-less-edge assertion). The
> legacy "receive a global via a matching param" path is *kept* as backward-compatible
> convenience (it's no longer load-bearing for the invariant), so adding `CliGlobals` is the
> only migration existing `global` consumers need.
>
> **Param-name scope decision:** `wire_name` is honored only on the CLI *flag* surface
> (clap arg id / `--long` / extraction key / slug positional). The `--params-json` and
> input/output-schema surfaces continue to key on the raw `name_str()` — they already used
> snake-case while flags used kebab, so this preserves the existing surface split rather
> than widening scope. Recorded as a deliberate, reviewable choice.
>
> **Decision kept, not subtracted:** per §8, we adopted `CliGlobals` (the minimal,
> name-referenced delivery hook) and did **not** pursue design-A (`render(mode)` subtraction
> of `display_with`). 3b therefore remains a partial footgun (CI-guardable only); the
> normalize-side `display_with`→`render` subtraction is the separable follow-on.
**Triggering case:** the `--pretty` footgun documented in
`normalize/docs/artifacts/sessions-stats-output-2026-06-20/` (`pretty-wiring-audit.md`,
`design-A-subtract.md`, `design-D-build-guard.md`, `judge-feasibility.md`). This doc
generalizes that single instance into the whole class and designs the invariant that
makes the class statically impossible.

---

## 1. The footgun class, stated generally

A `#[cli]` capability is a thing the macro lets a consumer **declare** (an impl-level
`#[cli(...)]` arg, a method-level `#[cli(...)]` attribute, or a `#[param(...)]` attribute).
Every capability has two halves:

- **the advertisement** — what the macro emits into the clap `Command` / `--help` / the
  generated dispatch (the flag exists, the subcommand exists, the arg is shown);
- **the termination** — the code that actually makes the advertised thing *do* its job.

The bug class is: **the advertisement is emitted unconditionally, but the termination
depends on consumer code (or a consumer convention) the macro neither emits nor verifies.**
When the consumer omits the termination, the capability is *advertised but silently inert*:
no compile error, the flag/arg/attribute is accepted, and it does nothing (or the wrong
thing) at runtime.

The lens for classifying every capability is **how the termination is bound**:

| binding | termination is… | omission is… | verdict |
|---|---|---|---|
| **macro-terminated** | emitted by the macro itself, from one declaration | impossible (there is nothing to omit) | **SAFE** |
| **name-referenced** | a consumer item named *in the attribute* (`display_with = "fn"`) | a **compile error** (unresolved path / trait bound / arity) | **SAFE-ish** (see §4) |
| **convention-referenced** | a consumer item the macro expects *by convention*, not named anywhere (a matching `pretty` param + a `Cell` + a display fn that reads it) | **silent** | **FOOTGUN** |
| **ignored** | the attribute parses but this projection never reads the field | **silent** | **FOOTGUN** |

`--json`/`--jsonl`/`--jq`/`--params-json`/`--manual`/`--input-schema`/`--output-schema`
are the proof that **macro-terminated** is achievable: the macro registers the flag
(`cli.rs:1088-1118`, `1061-1087`) *and* emits the code that consumes it in the same
generated arm (`gen_value_display` at `cli.rs:1949-1960`; the schema/manual arms at
`cli.rs:2036-2055`). The consumer writes no glue, so there is zero defect surface — and
correspondingly zero reported defects for those flags. **The whole invariant is: make
every capability macro-terminated or name-referenced; never convention-referenced or
ignored.**

---

## 2. The capability catalogue

Every capability the `#[cli]` macro exposes, classified. Sources: `CliArgs` parse
(`cli.rs:162-300`), the method-attr probes (`cli.rs:308-510`), and `parse_param_attrs`
(`parse/lib.rs:585-742`).

### 2a. Impl-level `#[cli(...)]` args

| capability | where parsed | classification | notes |
|---|---|---|---|
| `name`, `version`, `description`, `homepage`, `description_prefix` | `cli.rs:197-239` | **macro-terminated** | pure metadata → clap `Command` |
| `no_sync`, `no_async` | `cli.rs:171-184` | **macro-terminated** | gate generated entrypoints |
| `manual`, `input_schema`, `output_schema` (default on) | `cli.rs:240-251` | **macro-terminated** | registered *and* consumed by the macro — the model pattern |
| `defaults = "fn"` | `cli.rs:232-235` | **name-referenced** | emits `self.#fn(name)` (`cli.rs:1779,1789`); missing/ill-typed fn → compile error. One residual silent edge — see §3d. |
| `config_ty` / `config_cmd_name` (via `#[program(config=…)]`) | `cli.rs:1222-1232` | **macro-terminated** | feature-gated, fully generated |
| **`global = [flag, flag = "help"]`** | `cli.rs:214-231` | **CONVENTION-REFERENCED → FOOTGUN** | the central defect. See §3a. |

### 2b. Method-level `#[cli(...)]` attributes

| capability | where | classification | notes |
|---|---|---|---|
| `name` | `parse/lib.rs:362-387` | **macro-terminated** | subcommand rename |
| `skip` / `helper` | `cli.rs:308-330` | **macro-terminated** | excluded from partition |
| `hidden` | `cli.rs:458-480` | **macro-terminated** | `.hide(true)` |
| `default` | `cli.rs:333-352` | **macro-terminated** | + `compile_error!` on >1 default (`cli.rs:787-797`) |
| `manual = false` | `cli.rs:357-379` | **macro-terminated** | excluded from manual aggregate |
| **`display_with = "fn"`** | `cli.rs:483-510` | **name-referenced, opaque body → PARTIAL FOOTGUN** | the fn is *called* (`cli.rs:1924`) so a missing fn is a compile error; but its body is invisible to the macro, so it can silently ignore the flags it is supposed to honor. See §3b. |

### 2c. Subcommand / mount registration (return-type driven)

| capability | where | classification | notes |
|---|---|---|---|
| leaf (value return) | `parse/lib.rs:1173-1199` | **macro-terminated** | |
| static/slug mount (`-> &T`) | `cli.rs:1373-1448`, `2282-2396` | **name-referenced** | mounted `T` must impl `CliSubcommand` → trait-bound compile error if not |
| `Result` / `Option` / `Iterator` / unit rendering | `cli.rs:1964-2024` | **macro-terminated** | error → `eprintln!` + `exit(1)`; iterator streams jsonl. Rigid but never silent. |
| async vs sync dispatch | `cli.rs:1866-1896` | **macro-terminated** | + await-without-async is a compile error (`parse/lib.rs:278-290`) |

### 2d. Param-level `#[param(...)]` attributes — **as seen by the CLI projection**

| `#[param(...)]` | `ParamInfo` field | read by CLI codegen? | classification |
|---|---|---|---|
| `short = 'x'` | `short_flag` | yes (`cli.rs:1528`) | **macro-terminated** |
| `help = "..."` | `help_text` | yes (`cli.rs:1550-1606`) | **macro-terminated** |
| `positional` | `is_positional` | yes (`cli.rs:1354,1576`) | **macro-terminated** |
| `query`/`path`/`body`/`header` | `location` | n/a — HTTP-only | not a CLI capability (correctly inert) |
| `env`/`file_key`/`nested`/`serde`/`env_prefix` | — | n/a — `#[derive(Config)]`-only | not a CLI capability |
| **`name = "..."`** | `wire_name` | **NO** | **IGNORED → FOOTGUN** (§3c) |
| **`default = ...`** | `default_value` | **NO** | **IGNORED → FOOTGUN** (§3c) |

`aliases` and `flatten` were called out in the audit brief: **neither exists** as a `#[cli]`
capability (grep of `cli.rs` finds no alias/flatten handling; `flatten` is an HTTP/Config
concept). So they cannot be footguns here — there is nothing to advertise. Recorded as a
negative finding.

---

## 3. The footguns, with evidence

### 3a. `global = [...]` — convention-referenced delivery (THE class exemplar)

**Silent failure:** declare `#[cli(global = [pretty, compact])]`; `--pretty` appears in
`--help` and parses; but unless the method *also* (i) redeclares `pretty: bool, compact: bool`
params, (ii) the body mutates some `self.pretty` state, and (iii) a `display_with` fn reads
that state, the flag does nothing. Omit any link → silent text fallback. The audit found
**8 live BROKEN commands** in normalize this way.

**Evidence — advertisement without termination:**
- Advertisement: every `global` entry becomes a clap arg on the root, `.global(true)`,
  with no consumption code (`cli.rs:1003-1021`).
- Delivery is *convention only*: in `generate_leaf_match_arm`, a global flag's value is
  read **iff the method declared a matching param** (`cli.rs:1715-1732`, gated on
  `is_global` at `1720`); a global the method did not declare is filtered out of the
  subcommand's args (`cli.rs:1343-1349`) and its value is never bound.
- The method call passes **only declared params** (`arg_names` →
  `self.#method_name(#(#arg_names),*)`, `cli.rs:1814,1893`).
- The display fn is invoked with no flag context (`self.#display_fn(&value)`, `cli.rs:1924`).

So the value of a `global` flag reaches the body only through a re-declared param, and even
then the *effect* is entirely hand-written. Nothing in the macro emits, references, or
checks that effect. This is the textbook **convention-referenced** termination.

### 3b. `display_with = "fn"` — name-referenced but opaque

**Silent failure:** the named fn is guaranteed to be *called* (so a missing or mis-typed fn
is a compile error — this is the safe part), but the macro cannot see its body. A
`display_with` fn that calls `format_text()` unconditionally (ignoring whatever flag it was
meant to honor) renders a real `format_pretty()` dead. The audit's "dead dispatch" cases
(`edit` ×6, `syntax node-types`) are exactly this.

**Evidence:** `gen_value_display` emits `let __display = self.#display_fn(&value);` with no
mode/flag argument (`cli.rs:1922-1926`). The macro's contract ends at "your fn was called
with the value"; correctness of what the fn *does* is outside its reach.

This is a **partial** footgun: the *existence* of the sink is statically guaranteed; only
its *behavior* is not. It cannot be closed by a guard (the body is opaque); it is closed
only by removing the hand-written bridge entirely (§5, design-A route).

### 3c. `#[param(name = "...")]` and `#[param(default = ...)]` — ignored in CLI

**Silent failure (name):** annotate a param `#[param(name = "q")]`; the flag is still
advertised and extracted as `--query` (the kebab of the Rust identifier), not `--q`. The
rename is silently dropped.

**Silent failure (default):** annotate `#[param(default = 10)]`; the value is not applied
and the param is not made optional — a required param with a `default` still errors with
"Missing required argument" when omitted.

**Evidence (verified by grep of `cli.rs`):**
- `wire_name` is read **only for method/subcommand naming** (`cli.rs:512-515`,
  `wire_name_or`). Param arg generation and extraction use `param.name_str().to_kebab_case()`
  throughout (`cli.rs:1526`, `1717`, `2335`) — never `param.wire_name`.
- `default_value` has **zero** references in `cli.rs` (grep returns nothing). The CLI's only
  default mechanism is the impl-level `defaults = "fn"` resolver (`cli.rs:1771-1794`); the
  per-param `#[param(default)]` field (`parse/lib.rs:611-628`) is parsed and discarded.

These are the purest form of the class: the attribute parses without complaint
(`parse_param_attrs` accepts them, `parse/lib.rs:605,611`) and the projection never reads
the field. Contrast HTTP/Config, which *do* read them — so this is a CLI-projection gap,
not a parser bug.

### 3d. `defaults = "fn"` — one silent edge

Mostly safe (name-referenced: a missing fn is a compile error). Residual: the resolver is
only consulted for **required, non-optional, non-bool, non-vec** params (`cli.rs:1771`, the
final `else if let Some(defaults_fn)` branch). Declare `defaults = "fn"` on an impl whose
leaf params are all `Option<_>` and the fn is never called — advertised-but-inert, silent.
Low severity (rare shape), but it is the same class.

---

## 4. Why "name-referenced" is the bar, and the invariant

The catalogue shows a clean split. **Macro-terminated** capabilities have *no* defect
surface. **Name-referenced** capabilities (`display_with`, `defaults`, mounts) push the
"did you supply the sink?" question onto the *compiler*: the sink is named in the source,
so its absence or wrong shape is `E0277`/`E0599`/arity error — loud, not silent. The
**only** silent capabilities are the **convention-referenced** one (`global`) and the
**ignored** ones (`#[param(name/default)]` in CLI).

> **The invariant:** *server-less must have no convention-referenced or ignored CLI
> capability.* Every capability the macro advertises must be terminated by either
> (a) macro-generated code (one declaration, no consumer glue), or (b) a sink **named in
> the declaration** whose absence/mismatch is a compile error. A capability that is
> advertised but whose termination is neither generated nor named is a framework defect.

Equivalent operational phrasing for the macro author: **the macro must not emit an
advertisement it cannot itself terminate or whose terminator it does not name.** Today
`global` violates the first clause (advertised, not terminated, terminator unnamed) and
`#[param(name/default)]` violate it by being advertised-as-parsed but never read.

### The single general mechanism: a register↔consume ledger

The highest-value outcome is **not** N point fixes but one structural rule the macro
enforces once. The macro already has, at expansion time, the complete set of flag ids it
registers (the format flags, the gated meta flags, and `global_flags` from `cli.rs:608`).
Make that set a **ledger** with the invariant: *every registered flag id is either consumed
by macro-generated code in the dispatch arm, or routed to a named/typed sink.* Concretely:

1. **Built-in flags** (json/jsonl/jq/params-json/manual/schema): already consumed by the
   macro → ledger-satisfied, unchanged.
2. **`global` flags**: the macro must generate a **delivery path** to a single typed sink it
   *names by type* — a `CliGlobals`-style trait the service implements once. The macro emits
   `<Self as ::server_less::CliGlobals>::set_global_flag(self, "pretty", sub_matches.get_flag("pretty"))`
   before the call, for each declared bool global. Because the sink is named (a trait
   bound), omitting it is a compile error (§6 probe). This converts `global` from
   convention-referenced to **name-referenced**, and the per-method param/`Cell`/resolve
   chain disappears — there is nothing left to forget.

This is general because it is stated over *the registration set*, not over `pretty`
specifically: **any** future `global` flag is delivered by the same generated path and
checked by the same bound. It closes 3a for all flags at once, and the same ledger discipline
forbids adding a new registered flag without a consumer — preventing the class from
re-emerging.

---

## 5. Per-footgun remedy

| footgun | remedy | becomes |
|---|---|---|
| **3a `global` delivery** | the ledger mechanism: macro generates delivery to a named `CliGlobals` sink; **no blanket default impl** (a default no-op *is* a silent sink — it would re-create the footgun). Declaring `global` ⇒ `Self: CliGlobals` enforced by a generated bound. | **name-referenced** (compile error if unwired) |
| **3a effect (config/TTY resolution)** | resolution moves into the *one* sink method, written once per service, not per leaf body. Server-less stays ignorant of `NormalizeConfig`; it owns *dispatch*, the consumer owns *policy*. | macro-terminated dispatch + name-referenced policy |
| **3b `display_with` opaque body** | cannot be guarded (body opaque). Two routes: **(A, preferred long-term)** subtract it — replace `display_with` + `Cell` with one macro-driven `render(mode)` call (design-A); the bridge ceases to exist, so "dead dispatch" is unrepresentable. **(fallback)** a behavioral CI snapshot test (design-D Layer 3). | (A) macro-terminated; (fallback) CI-detected |
| **3c `#[param(name)]`** | **honor it**: read `param.wire_name` in `generate_arg` and in all extraction sites instead of `name_str()`. Purely additive codegen fix. | macro-terminated |
| **3c `#[param(default)]`** | **honor it** (apply `default_value` to clap `.default_value(...)` / make the param optional) **or** emit a `compile_error!` "`#[param(default)]` is not supported by the CLI projection; use `#[cli(defaults = \"fn\")]`". Honoring is better (consistent with HTTP/Config). | macro-terminated (or loud) |
| **3d `defaults` on all-optional impl** | low priority; a `compile_error!` "`defaults = \"fn\"` declared but no required param consumes it" closes it. | name-referenced/loud |

The ordering that matters: **3c is the cheapest and is unambiguous** (pure additive bug
fixes, no consumer migration, no cross-crate coordination). **3a is the structural
centerpiece** (the ledger + `CliGlobals`). **3b is the deepest** and is the subject of the
four normalize design docs; it needs the human call in §8.

---

## 6. Feasibility (rustc-probed, toolchain active)

`rustc 1.95.0`, `--edition 2021`. Probes under the session scratchpad.

- **Ledger compile-error for `global` (3a) — VERIFIED.** A generated bound
  `fn _assert<T: CliGlobals>()` instantiated at the service type fails to compile when the
  service does not implement `CliGlobals`:
  ```
  error[E0277]: the trait bound `Svc: CliGlobals` is not satisfied
  help: the trait `CliGlobals` is not implemented for `Svc`
  ```
  and compiles cleanly once the impl is present. **Critical corollary the probe makes
  concrete:** this enforcement holds *only if `CliGlobals` has no blanket default impl*. A
  blanket `impl<T> CliGlobals for T` (proposed in the audit sketch for "backward compat")
  would make the bound always satisfied and the call silently resolve to the no-op —
  re-introducing the exact silent-inert state. **No blanket default.** Declaring `global`
  must require the explicit impl.
- **Token-level `compile_error!` for the param-presence variant (design-D) — already
  verified SOUND** by `judge-feasibility.md` (D verdict): the macro sees `global_flags` and
  every method's params at one expansion (`cli.rs:608` + `631` + `636`), with a working
  precedent in `check_reserved_flag_collisions` (`cli.rs:384`). No type info needed.
- **`#[param(name/default)]` honoring (3c) — trivially feasible**: pure codegen, the fields
  already exist on `ParamInfo`; no type resolution, no probe needed.
- **Distinguishing "macro-terminated" line:** compile-time enforcement covers 3a (delivery)
  and 3c (codegen). It does **not** cover 3b's *behavioral* correctness (opaque body) — that
  is CI/trybuild territory unless `display_with` is removed via design-A. This is the one
  honest hard line: presence is compile-checkable, *behavior of a named hand-written fn* is
  not.

---

## 7. Cost & blast radius (server-less is published, v0.5.0; no path deps)

| remedy | additive or breaking | cost to existing consumers |
|---|---|---|
| **3c honor `#[param(name)]`** | **behavior change** | any consumer relying on the (buggy) current behavior — a `#[param(name)]` that was silently ignored now changes the flag name. Likely zero real consumers depend on the bug, but it *can* rename a flag. Gate under a minor version; note in CHANGELOG. |
| **3c honor `#[param(default)]`** | **additive** | none — the field is currently discarded; honoring it only affects params that already carry the attribute and currently error. Strict improvement. |
| **3a `CliGlobals` + generated delivery** | **additive at the macro**, **breaking for declarers of `global`** | the macro change is additive (new trait + generated call + bound). But the bound makes existing `global = [...]` impls fail to compile until they `impl CliGlobals`. That is the *intended forcing function* (it converts the 8 silent bugs into 8 build breaks), but it is a breaking change for any consumer using `global`. Sequence: land in a minor bump, document the migration. |
| **3a no blanket default** | (part of above) | the cost *is* the breakage; a blanket default would avoid breakage but defeats the purpose. Accept the breakage. |
| **3b design-A (remove `display_with`)** | **breaking** | largest: changes the output-rendering contract for every consumer using `display_with`. Cross-repo coordination (publish server-less, then bump normalize). Defer behind the §8 decision. |
| **3b design-D fallback (CI test)** | additive (consumer-side test) | no framework breakage; pushes burden to consumer CI + fixtures. |

The cheapest high-value bundle: **3c (both) + the design-D Layer-1 `compile_error!` for the
param-presence variant of 3a**, all in one minor bump, no trait redesign. The full ledger
(`CliGlobals`) is the next step up and the real root-cause fix; design-A is a larger,
separable decision.

---

## 8. The open design question for a human

**For `global`-flag semantic capabilities (the `pretty` shape): adopt the `CliGlobals`
delivery hook (keep the runtime-state architecture, make it name-referenced), or subtract
the hand-written render bridge entirely (design-A `render(mode)`, make the bridge
unrepresentable)?**

- `CliGlobals` is the **minimal** change that makes 3a impossible-by-construction for
  *delivery*: additive macro support, a per-service trait impl, a compile-error bound. It
  does **not** touch `display_with` (3b stays a partial footgun, CI-guarded).
- design-A is **deeper**: it removes `display_with` and the `Cell` outright, making 3a *and*
  3b impossible-by-construction, restoring symmetry with the `--json` path. But it is a
  breaking change to the rendering contract for every consumer, with cross-repo sequencing.

The judge (`judge-feasibility.md`) found design-A SOUND-WITH-CAVEAT (a fixable `E0382` in
the `render_root` threading) and design-D SOUND with the lowest mechanism risk. Both are
viable; the choice is a cost/scope call (incremental hook now vs. one larger contract change)
that belongs to a human. This doc's contribution is the *invariant* both must satisfy:
**no convention-referenced or ignored capability** — and the ledger framing that makes the
choice about *how* to terminate `global`, not *whether* to.

---

## 9. Summary of negative findings (write-it-down)

- `aliases` and `flatten` are **not** `#[cli]` capabilities — no advertisement exists, so no
  footgun. (If they are added later, they must be born macro-terminated or name-referenced.)
- `#[param(query/path/body/header)]` and `#[param(env/file_key/nested/serde/env_prefix)]`
  being inert in CLI is **correct** — they are other-projection capabilities, not advertised
  by the CLI surface. Not footguns.
- The `Result`-error and `Option`-`None` rendering paths are rigid (always
  `eprintln!`+`exit(1)`) but **never silent** — macro-terminated, not in the class.
