---
id: rust-module-boundaries
aliases:
  - facade split
  - child module extraction
  - re-export closure
  - visibility drift
  - attribute drift
implemented_by:
  - crates/ambition_platformer2d_core/src
  - crates/ambition_platformer2d_actor_monolith/src
related_memory:
  - dev/journals/rust-module-split-import-visibility-lessons-2026-05-11.md
  - dev/journals/movement-refactor-lessons-2026-05-11.md
  - dev/benchmark-candidates/rust-questions.md
  - dev/benchmark-candidates/compositions.md
last_verified: 2026-09-03
---

# Rust module boundaries

## Definition

Rust module-boundary work includes splitting large facade files into private child modules, moving tests, changing public API re-exports, extracting helpers, and preserving derive/doc-comment adjacency.

## Core invariants

- Moving an item moves its attributes and doc comments with it.
- A `pub` item in a private child module is not visible through the facade unless re-exported.
- Sibling modules need explicit local imports or facade-visible helpers.
- Extension traits must be in scope at the call site after a split.
- `include_str!` and `include_bytes!` paths are relative to the source file containing the macro.
- Tests moved out of a module can strand attributes, fixtures, and helper visibility.

## Additional concerns when moving code between crates

The same module rules apply, but moving code between crates also changes
visibility, imports and dependency ownership:

- **`pub(in crate::some::path)` is crate-relative.** Update the visibility
  scope to a valid owner in the destination crate.
- **Rewrite imports according to what each path names.** A test module's
  `use super::*` still refers to its enclosing module; a blanket `super::` to
  `crate::` replacement breaks that relationship.
- **Distinguish real imports from text and macro paths.** Former external
  crate references may become `crate::` paths, but doc comments, log targets
  and relative `include_str!` paths must be handled separately.
- **Cross-crate integration tests belong in a consumer that depends on the
  participating crates.** Do not invert the package graph just to retain a
  test's old location.

Check the destination crate's existing dependency and visibility boundaries
before moving code into it.

## Edit protocol

1. Identify the destination owner and public facade surface.
2. Move one coherent item group at a time, keeping attributes and docs attached.
3. Correct crate-relative imports, visibility, fixtures and asset includes.
4. Keep facade re-exports intentional; remove obsolete forwarding paths.
5. Format and compile the affected crates. Consult the related `dev/` journal
   when the move resembles a known failure.

## Validation

```bash
cargo fmt --check
cargo check -p <destination-crate>
```

Add a focused test run if behavior changed or a specific regression needs
verification. A module move does not automatically warrant two full test lanes.
