# Lesson learned: Rust facade splits need item-complete chunks and re-export-visible helpers

## Symptom

After splitting `crates/ambition_gameplay_core/src/music/director.rs` into child modules, `cargo fmt --all` failed with:

```text
error: expected item after doc comment
  --> crates/ambition_gameplay_core/src/music/director/loader.rs:34:1
```

`cargo test` then failed with privacy errors such as:

```text
error[E0364]: `resolve_adaptive_directive` is private, and cannot be re-exported
error[E0603]: function import `resolve_adaptive_directive` is private
```

## Root cause

The extraction was syntactic rather than item-aware in two places:

1. `loader.rs` ended with a copied fragment of the `MusicDirector` doc comment, but the documented item stayed in `director.rs`. Rust doc comments are attributes; an orphaned `///` at EOF is not harmless text.
2. Functions in child modules were marked `pub(super)`, which exposed them only to `music::director`. The facade then attempted to re-export them as `pub(super)` from `music::director` to `music`, but the original item visibility was not broad enough for that re-export.

## Fix

- Remove the orphaned doc-comment fragment from `loader.rs`.
- Keep internal helpers restricted, but widen the specific helper functions needed by parent-module tests / test imports to `pub(in crate::music)`.

## Takeaway

When splitting a Rust module into a facade plus children, validate two boundaries explicitly:

1. **Item boundary:** every moved chunk must start and end on complete Rust items, including attributes and doc comments.
2. **Visibility boundary:** any helper re-exported through the facade must be visible at least as far as the facade's re-export target. `pub(super)` in a child is not enough if the facade re-exports to its own parent.
