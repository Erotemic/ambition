---
id: rust-module-boundaries
aliases:
  - facade split
  - child module extraction
  - re-export closure
  - visibility drift
  - attribute drift
implemented_by:
  - crates/ambition_engine/src
  - crates/ambition_sandbox/src
related_memory:
  - dev/journals/rust-module-split-import-visibility-lessons-2026-05-11.md
  - dev/journals/movement-refactor-lessons-2026-05-11.md
  - dev/benchmark-candidates/rust-questions.md
  - dev/benchmark-candidates/compositions.md
last_verified: 2026-05-17
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

## Edit protocol

1. Search `dev/benchmark-candidates/` for module-split traps before editing.
2. Move one coherent item group at a time.
3. Re-check imports inside each child module locally; do not rely on parent imports.
4. Keep facade re-exports intentional and stable.
5. Run `cargo fmt` after structural moves; it catches stranded attributes and parse drift early.

## Validation

```bash
cargo fmt --check
cargo test -p ambition_engine
cargo test -p ambition_sandbox --lib
```

Use narrower package/module tests while iterating, then run the package-level checks before handoff.
