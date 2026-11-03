# `ambition_load_presentation` — module map

<!-- BEGIN generated module map (scripts/measure/modules_md.py) -->

**ambition_load_presentation** — Replaceable, contributor-neutral presentation for unresolved load barriers.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`basic_presentation`](src/basic_presentation.rs) | Show load status and the controls needed to continue or recover. |
| [`deterministic_activity`](src/deterministic_activity.rs) | Optional deterministic loading activity acceptance fixture. |
| [`input`](src/input.rs) | Read loading controls from the participants that own the foreground. |
| [`model`](src/model.rs) | Load-foreground policy, semantic view model, and arbitrary activity protocol. |
| [`plugin`](src/plugin.rs) | Contributor-neutral hidden-grace, ready-hold, activity, and cleanup lifecycle. |
| [`shell_adapter`](src/shell_adapter.rs) | Thin adapter from shell-route lifecycle to contributor-neutral load presentation. |

_6 crate-root modules. Regenerate: `python scripts/measure/modules_md.py --write`._

<!-- END generated module map -->

## Notes

The core plugin is host-neutral. `AmbitionLoadShellPresentationPlugin` owns all
shell route holds, retry, cancellation, and navigation policy. Room transitions
must drive the core protocol directly rather than manufacturing shell routes.
