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

## ⭐ Moving a module to ANOTHER CRATE adds four traps these invariants do not name

Verified 2026-09-03 by carving `encounter/` out of the actor kernel into
`ambition_encounter_features`. Every invariant above held and three of them bit —
the `include_str!` path (one fewer `../` from the new depth), stranded test
helpers, and sibling imports. These four are additional, and all four are
mechanical:

- **`pub(in crate::some::path)` is CRATE-relative and silently wrong after the
  move.** `pub(in crate::encounter)` compiled for years and became "could not
  find `encounter` in the crate root" the moment the module had a new crate root.
- **A blanket `super::` → `crate::` rewrite corrupts intra-file test modules.**
  `mod tests { use super::*; }` means *this file's parent*, not the crate; the
  substitution turns a working test import into `use crate::*` and the file stops
  seeing its own private helpers. Rewrite the CROSS-module paths and leave
  `use super::*` alone.
- **The old crate's name inside the moved code is now an external crate.**
  `ambition_encounter::Foo` becomes `crate::Foo` — but a naive replace also
  rewrites the string inside `crate::crate::`, and doc comments and log TARGET
  strings that merely MENTION a crate are not dependencies at all. Three
  "impossible cycles" in this carve were a comment, an `include_str!` path and a
  `log` target.
- **A test that composes two plugins cannot live in the crate that can only name
  one.** Cross-crate ordering guards belong in whichever crate depends on both —
  usually the consumer — or the move makes the guard uncompilable and the
  tempting fix is to delete the half that does not fit.

⚠ And the destination is a decision, not a detail: this carve's first attempt
moved the module INTO `ambition_encounter`, and that crate's own policy rows
refused it. **Read the destination crate's policy rows before writing the move
down as a plan.**

## Edit protocol

1. Search `dev/benchmark-candidates/` for module-split traps before editing.
2. Move one coherent item group at a time.
3. Re-check imports inside each child module locally; do not rely on parent imports.
4. Keep facade re-exports intentional and stable.
5. Run `cargo fmt` after structural moves; it catches stranded attributes and parse drift early.

## Validation

```bash
cargo fmt --check
cargo test -p ambition_platformer2d_actor_monolith
cargo test -p ambition_platformer2d_actor_monolith --lib
```

Use narrower package/module tests while iterating, then run the package-level checks before handoff.
