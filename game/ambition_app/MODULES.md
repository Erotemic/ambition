# `ambition_app` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_app** — Ambition app shell (Stage 20 / A3 bisection).

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`app`](src/app/mod.rs) | Sandbox app-builder: domain plugins, helpers, and gameplay systems shared between the visible binary (`src/bin/ambition_game_bin.rs`) and headless drivers (`src/headless.rs`, `src/rl_sim/runtime.rs`). |
| [`dev`](src/dev/mod.rs) | App-level developer presentation: F1 debug overlay, F3 FPS counter, and the F9 one-shot GGRS rollback proof with a platform-neutral control resource. |
| [`headless`](src/headless.rs) | Headless simulation entry point. |
| [`host`](src/host/mod.rs) | Host-platform integration: per-OS plugin selection (desktop, android, …) and window/display-mode controls. |
| [`menu`](src/menu/mod.rs) | Game-side menu host stack: backend-agnostic page model, dispatcher, item effects, and the flat-grid / 3D-cube presentation hosts. |
| [`rl_sim`](src/rl_sim/mod.rs) | Ambition's binding of the reusable [`ambition_sim_harness`] to its own content. |

_6 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
