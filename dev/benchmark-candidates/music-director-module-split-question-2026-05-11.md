# Benchmark candidate: Rust module splits need item-complete extraction and sufficient re-export visibility

## Context

During the `music/director.rs` split, a child module was created for cue loading and private helper modules were re-exported through the new `director.rs` facade. The overlay failed before tests could run:

```text
error: expected item after doc comment
  --> crates/ambition_actors/src/music/director/loader.rs:34:1

error[E0364]: `resolve_adaptive_directive` is private, and cannot be re-exported
error[E0603]: function import `resolve_adaptive_directive` is private
```

The root causes were:

1. The extraction left a trailing `///` doc comment fragment in `loader.rs` with no item following it.
2. Helpers defined in `director/resolver.rs` and `director/adaptive.rs` were only `pub(super)` to their immediate parent module, but `director.rs` tried to re-export them to its own parent module for sibling tests / test-only imports.

## Benchmark question

You are splitting a Rust file `music/director.rs` into child modules:

```text
music/director.rs
music/director/loader.rs
music/director/resolver.rs
music/director/adaptive.rs
```

The parent module contains:

```rust
#[cfg(test)]
pub(super) use adaptive::should_restart_adaptive;
#[cfg(test)]
pub(super) use resolver::{resolve_adaptive_directive, resolve_directive_for_binding};
```

The child modules define those functions as:

```rust
pub(super) fn should_restart_adaptive(...) -> bool { ... }
pub(super) fn resolve_adaptive_directive(...) -> Option<AdaptiveCueDirective> { ... }
pub(super) fn resolve_directive_for_binding(...) -> Option<AdaptiveCueDirective> { ... }
```

A compile then reports that the functions are private and cannot be re-exported. Separately, `rustfmt` reports `expected item after doc comment` in `loader.rs`.

What should the refactor change before handing off the overlay?

## Expected answer

The extraction should first verify that every moved chunk is item-complete: no orphaned attributes or doc comments may remain in a child file. Delete or move the trailing doc-comment fragment in `loader.rs` so every `///` documents an actual following item.

For the re-export, visibility must be at least as broad as the facade re-export. `pub(super)` inside `director/resolver.rs` only exposes the function to `director`, not to `music`, so `director.rs` cannot re-export it as `pub(super)` to `music`. Use a restricted visibility that reaches the intended parent, for example:

```rust
pub(in crate::music) fn resolve_adaptive_directive(...) -> Option<AdaptiveCueDirective> { ... }
pub(in crate::music) fn resolve_directive_for_binding(...) -> Option<AdaptiveCueDirective> { ... }
pub(in crate::music) fn should_restart_adaptive(...) -> bool { ... }
```

or choose another visibility boundary that exactly matches the public/test surface needed by the parent module.

## What this tests

- Whether the assistant checks extraction boundaries for orphaned comments / attributes.
- Whether it understands Rust visibility relative to nested modules.
- Whether it distinguishes "visible to immediate parent" from "re-exportable to grandparent".
- Whether it keeps the facade API stable without unnecessarily making helpers globally public.
