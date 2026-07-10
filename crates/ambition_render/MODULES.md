# `ambition_render` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_render** — Ambition's Bevy presentation layer — the sandbox's default renderer.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`cutscene`](src/cutscene/mod.rs) | Sandbox cutscene presentation overlay. |
| [`dialog_ui`](src/dialog_ui.rs) | The dialog-box overlay UI: spawns/refreshes the on-screen dialog panel. |
| [`fx`](src/fx.rs) | Procedural visual effects for the sandbox. |
| [`hud`](src/hud.rs) | Always-on player HUD: health, mana, and money meters (visible build). |
| [`platformer_presentation`](src/platformer_presentation.rs) | **The presentation face a demo can add** — [`PlatformerPresentationPlugin`]. |
| [`quality`](src/quality.rs) | Live resolved visual-quality resource. |
| [`rendering`](src/rendering/mod.rs) | Bevy visual synchronization for engine state. |
| [`screen_effects`](src/screen_effects.rs) | Whole-screen post-processing effects for presentation cameras. |
| [`ui_fonts`](src/ui_fonts.rs) | UI font loading for the presentation layer. |

_9 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
