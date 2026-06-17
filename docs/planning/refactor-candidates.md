# Refactor candidates

This file tracks structural cleanup candidates. It is planning material, not a recipe. Turn an item into a focused patch only when there is a clear validation command and rollback boundary.

## Rules

- Search `dev/` for prior traps before broad module moves.
- Prefer one module family per patch.
- Preserve compatibility re-exports until downstream imports are updated.
- Regenerate `.agent/` indexes after moves.
- Do not mix gameplay behavior changes with mechanical file moves unless a test requires it.

## Current candidate areas

| Area | Candidate cleanup | Validation |
|---|---|---|
| Sandbox runtime phases | Continue promoting inline `sandbox_update` phases into named Bevy systems. | `cargo test -p ambition_gameplay_core --lib` plus focused gameplay tests. |
| Content feature modules | Retire compatibility references to old root `features` paths once call sites use `content/features`. | `cargo test -p ambition_gameplay_core content_validation` and conversion tests. |
| Trace/dev tools | Keep trace recorder under `crates/ambition_gameplay_core/src/dev/trace/`; remove stale docs that mention root `trace.rs`. | `cargo test -p ambition_gameplay_core trace`. |
| Settings/UI | Keep settings under `persistence/settings` and route UI through the unified menu stack. | `cargo test -p ambition_gameplay_core settings menu` plus app menu tests. |
| Architecture docs | Keep `docs/systems/architecture.md` as the consolidated system overview. | `python scripts/check_doc_links.py`. |

Promote resolved lessons to `dev/journals/` or concise current docs; do not leave patch-era prose in `docs/systems/` or `docs/recipes/`.
