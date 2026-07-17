# `ambition_load_presentation` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_load_presentation** — Replaceable, contributor-neutral presentation for unresolved load barriers.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`basic_presentation`](src/basic_presentation.rs) | Plain Bevy UI reference presentation for load evidence and ready-hold. |
| [`deterministic_activity`](src/deterministic_activity.rs) | Optional deterministic loading activity acceptance fixture. |
| [`model`](src/model.rs) | Load-foreground policy, semantic view model, and arbitrary activity protocol. |
| [`plugin`](src/plugin.rs) | Contributor-neutral hidden-grace, ready-hold, activity, and cleanup lifecycle. |
| [`shell_adapter`](src/shell_adapter.rs) | Thin adapter from shell-route lifecycle to contributor-neutral load presentation. |

_5 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

The core plugin is host-neutral. `AmbitionLoadShellPresentationPlugin` owns all
shell route holds, retry, cancellation, and navigation policy. Room transitions
must drive the core protocol directly rather than manufacturing shell routes.
