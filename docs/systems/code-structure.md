# Code structure

This page records the current module-shape rules agents should follow. It is
not a history of every split that led here; older notes are archived in
[`../archive/old-system-notes/code-structure-pre-kb-cleanup.md`](../archive/old-system-notes/code-structure-pre-kb-cleanup.md).

## Current rule: stable facade files

When splitting a large Rust module, keep the existing public file as a stable
facade and move implementation into child modules:

```text
foo.rs          # public facade, re-exports, high-level module docs
foo/*.rs        # focused implementation modules
```

Prefer this over replacing `foo.rs` with `foo/mod.rs`. It avoids overlay cleanup
problems, keeps imports stable, and makes review easier.

Current examples:

- `ambition_engine::movement` keeps `movement.rs` as the facade over
  `movement/` child modules.
- `ambition_sandbox::{app, audio, encounter, input, music, trace, dialog}` use
  facade modules over focused child files.
- `ambition_sandbox::ui_nav` is a shared folder-backed helper module for menu,
  dialog, inventory-like, and touch-navigation list behavior.

## Refactor checklist

Before handing off a module split:

1. Compare the old public surface with the new facade `pub use` surface.
2. Search dev memory for module-split traps:

   ```bash
   rg -n "module split|re-export|visibility|extension trait|Self: Sized" dev
   ```

3. Run focused tests for the touched module and one broader crate test.
4. Update the relevant concept/system doc when the new module shape becomes a
   durable navigation fact.

See also:

- [`../concepts/rust-module-boundaries.md`](../concepts/rust-module-boundaries.md)
- [`engine-architecture.md`](engine-architecture.md)
- [`../dev/benchmark-candidates/rust-module-split-subtle-review-question-2026-05-11.md`](../../dev/benchmark-candidates/rust-module-split-subtle-review-question-2026-05-11.md)
