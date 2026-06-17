# Benchmark candidate: `pub use` of a `pub(crate)` item silently re-exports past its visibility

Date: 2026-05-19

## Prompt

You are splitting `crates/ambition_gameplay_core/src/features/ecs/mod.rs`
into focused submodules and need to keep the existing call sites in
`app/plugins.rs`, `world_flow.rs`, etc. working without touching them.
The original `mod.rs` had two `pub(crate) fn` helpers that several
sibling modules consume:

```rust
// in mod.rs (before)
pub(crate) fn actor_component_snapshot(actor: &ActorRuntime) -> (...) { ... }
pub(crate) fn sync_actor_components_from_runtime(actor: &ActorRuntime, ...) { ... }
```

You extract `ActorRuntime` and these helpers into a new submodule
`ecs/actors.rs`. To preserve the existing import surface from the
sibling modules, you propose:

```rust
// in mod.rs (after)
mod actors;
pub use actors::{
    actor_component_snapshot,        // ← BUG
    sync_actor_components_from_runtime,  // ← BUG
    update_ecs_actors,
    ActorRuntime,
};
```

Compile this with `cargo check -p ambition_gameplay_core --lib`. What does
the compiler say, and what's the minimum fix? Be specific about which
item carries which visibility and why a `pub use` re-export differs
from an inline declaration.

## Reference answer

The compiler emits `E0364` twice:

```
error[E0364]: `actor_component_snapshot` is only public within the crate,
              and cannot be re-exported outside
error[E0364]: `sync_actor_components_from_runtime` is only public within
              the crate, and cannot be re-exported outside
```

Cause: in the new `actors.rs` the helpers are declared `pub(crate) fn …`.
Visibility of an item in a `use` re-export is the minimum of the item's
own visibility and the visibility of the `use`. `pub use` advertises the
binding as `pub`, but a `pub(crate)` source is only visible within the
crate, so the re-export would let external callers see a binding for an
item they cannot actually reach. Rust rejects that at the re-export
rather than silently degrading.

Fix: match the visibilities. The minimum-churn change is to use
`pub(crate) use` for the helpers and `pub use` for the items that are
genuinely public:

```rust
pub use actors::{update_ecs_actors, ActorRuntime};
pub(crate) use actors::{actor_component_snapshot, sync_actor_components_from_runtime};
```

Equivalent fix: promote the helpers to `pub fn` in `actors.rs`. That
works but widens the API surface beyond what was originally exposed,
so prefer the `pub(crate) use` form when the original declarations
were `pub(crate)`.

## Why this matters as a benchmark

- Rust's E0364 only fires on `use` re-exports, not on direct declarations.
  An LLM that reasons "this used to be `pub(crate)`, so I'll use
  `pub(crate)` again at the site I'm migrating to" gets the right
  answer for free; one that reasons "submodule split = `pub use`"
  silently widens the visibility ladder.
- The fix is one keyword (`pub` → `pub(crate)` on the `use`), but only
  if the candidate notices the asymmetry. A reflex "make it `pub` in
  the new module" answer widens the API and is a long-term review
  hazard.
- Tests pass after either fix (the project compiles), so this is a
  pure visibility/policy question — easy to grade by inspecting the
  diff.
