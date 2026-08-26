# `ambition_render` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_render** — Ambition's Bevy presentation layer — the sandbox's default renderer.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`asset_census`](src/asset_census.rs) | Text census of texture decoding, in the style of the `[startup]` and `[schedule-census]` loggers in `ambition_dev_tools::profiling`. |
| [`capture`](src/capture.rs) | Generic offscreen capture plumbing for composed apps. |
| [`cutscene`](src/cutscene/mod.rs) | Sandbox cutscene presentation overlay. |
| [`dialog_ui`](src/dialog_ui.rs) | Provider-selectable dialogue presentation. |
| [`fx`](src/fx.rs) | Procedural visual effects for the sandbox. |
| [`gameplay_surround`](src/gameplay_surround.rs) | Design of record: `docs/planning/triage/gameplay-presentation-profiles.md`. |
| [`hud`](src/hud.rs) | Always-on player HUD: health, mana, and money meters (visible build). |
| [`platformer_presentation`](src/platformer_presentation.rs) | Generic platformer room presentation. |
| [`quality`](src/quality.rs) | Live resolved visual-quality resource. |
| [`reading_layout`](src/reading_layout.rs) | Where a block of text goes, for every overlay that shows one. |
| [`rendering`](src/rendering/mod.rs) | Bevy presentation systems that project simulation/view state into visuals. |
| [`screen_effects`](src/screen_effects.rs) | Whole-screen post-processing effects for presentation cameras. |
| [`ui_fonts`](src/ui_fonts.rs) | UI font loading for the presentation layer. |

_13 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
