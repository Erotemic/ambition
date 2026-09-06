# Lesson learned: Rust module splits need local scope review

Date: 2026-05-11

A repeated failure mode during the Ambition refactor sequence was assuming that
moving code from a parent module into child modules is mostly textual. In Rust,
each child module has its own name-resolution boundary. A child file using
`use super::*` may still compile for many sibling references, but it should not
replace a deliberate review of the child module's local import and visibility
closure.

Concrete examples from the body/mobile/map split:

- Extracted `body_mode/tests.rs` used Bevy symbols `App`, `Time`, and `Update`
  without importing them locally.
- `map_menu.rs` attempted to re-export `ui::short_room_label`, but the helper in
  `ui.rs` was private.

The fix pattern is to review each extracted child module before handoff:

1. Identify unqualified framework/prelude names and import them locally in the
   child module, especially in `tests.rs`.
2. Identify extension-method calls and ensure the trait is imported locally.
3. For each facade `use` or `pub use`, verify the source item is visible enough
   for the re-export.
4. Prefer the narrowest visibility that satisfies the actual access path:
   private `use`, `pub(super)`, or `pub(in crate::some_module)` before `pub`.

The benchmark version of this lesson should not announce the failure mode in the
prompt. It should provide a realistic, noisy refactor context with representative
snippets and ask for pre-handoff compile-risk review.
