# Mount Points

A mount point is a method that returns `&T` where `T: CliSubcommand`. The `#[cli]` macro detects these by return type and treats them as subcommand group delegation rather than leaf commands.

```rust
#[cli(name = "app")]
impl MyApp {
    /// Normal leaf command
    fn health(&self) -> String { "ok".to_string() }

    /// Mount: delegates to UserService's subcommand tree
    fn users(&self) -> &UserService { &self.users }
}
```

`app health` calls the leaf. `app users <subcommand>` descends into `UserService`'s command tree.

## Static vs Slug Mounts

**Static mounts** take no parameters beyond `&self`. The method name becomes the subcommand group:

```rust
fn users(&self) -> &UserService { &self.users }
// → app users list
// → app users edit --name "Alice"
```

**Slug mounts** take additional parameters. Those become positional arguments that appear before the nested subcommand:

```rust
fn user(&self, id: String) -> &UserService { ... }
// → app user <ID> list
// → app user <ID> edit --name "Alice"
```

The slug parameters are extracted and passed to the method before dispatch continues into the child's command tree. This maps naturally to resource-oriented CLIs: `app user 42 edit` reads as "on user 42, run edit."

## Why Return Type, Not Attributes

The obvious alternative would be an attribute:

```rust
#[cli(mount = "users")]
fn users(&self) -> &UserService { ... }
```

But `&T` already carries the meaning. In Rust, returning a reference to a value signals "I'm giving you access to something I own." That's exactly what delegation means here — the parent owns or holds the child and grants access to it. The return type is the annotation.

This keeps the common case annotation-free. You write what the method does; the derive figures out what it means. No new syntax to learn, no divergence between what the method signature says and what the attribute claims.

It also composes: if a third party adds a method returning `&T` to compose with your service, it just works without knowing anything about `#[cli]`.

## Deep Nesting

Nesting is recursive. Each `T` in a mount point is itself a `CliSubcommand`, so it can have its own mounts:

```rust
#[cli(name = "comments")]
impl CommentService {
    fn list(&self) -> Vec<String> { ... }
}

#[cli(name = "nested-posts")]
impl NestedPostService {
    fn list(&self) -> Vec<String> { ... }
    fn comments(&self) -> &CommentService { ... }
}

#[cli(name = "deep-app")]
impl DeepApp {
    fn posts(&self) -> &NestedPostService { ... }
}
```

This produces:

```
deep-app posts list
deep-app posts comments list
```

Three levels, no extra configuration. The tree emerges from the type structure.

## Opting Out

If a method returns `&T` but you don't want it exposed as a subcommand group, use `#[cli(skip)]`:

```rust
#[cli(skip)]
fn internal(&self) -> &UserService { &self.internal }
```

The method is left in the impl block untouched; it just isn't projected into the CLI. This is the standard server-less escape hatch: the convention applies by default, skip when it doesn't fit.

## The Tradeoff

The approach is less explicit than an attribute — you can't tell at a glance without knowing the convention that `-> &T` means "subcommand group." The documentation and the compiler error messages are the answer to that: the convention is simple enough to state in one sentence, and anyone reading code that uses `#[cli]` will encounter it quickly.

The payoff is that delegation composes naturally with the rest of Rust. Static analysis tools, documentation generators, and `cargo expand` output all reflect the actual type structure. There's no divergence between the method's type and its CLI behavior.
