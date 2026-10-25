# `ambition_render` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_render** — Ambition's Bevy presentation layer: the sandbox's default renderer.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`asset_census`](src/asset_census.rs) | Text census of texture decoding, in the style of the `[startup]` and `[schedule-census]` loggers in `ambition_dev_tools::profiling`. |
| [`bubble_shield_fx`](src/bubble_shield_fx.rs) | The live layer of the bubble shield. |
| [`capture`](src/capture.rs) | Generic offscreen capture plumbing for composed apps. |
| [`cutscene`](src/cutscene/mod.rs) | Sandbox cutscene presentation overlay. |
| [`dialog_ui`](src/dialog_ui.rs) | Provider-selectable dialogue presentation. |
| [`folded_mesh`](src/folded_mesh.rs) | The one road for a mesh built on the CPU: lightning, lasers, any line or ribbon an effect lays out as triangles. |
| [`fx`](src/fx.rs) | Procedural visual effects for the sandbox. |
| [`gameplay_surround`](src/gameplay_surround.rs) | Fills the display area that the gameplay camera does not cover. |
| [`gate_portal_fx`](src/gate_portal_fx.rs) | The wormhole of a gate, at Ultra. |
| [`hud`](src/hud.rs) | Always-on player HUD: health, mana, and money meters (visible build). |
| [`lines`](src/lines/mod.rs) | Line effects: lightning, lasers, any look drawn along a line or round a body. |
| [`mesh2d_late_meshes`](src/mesh2d_late_meshes.rs) | A 2D mesh whose GPU mesh arrives late is specialized when it arrives. |
| [`platformer_presentation`](src/platformer_presentation.rs) | Generic platformer room presentation. |
| [`quality`](src/quality.rs) | Live resolved visual-quality resource. |
| [`reading_layout`](src/reading_layout.rs) | Where a block of text goes, for every overlay that shows one. |
| [`rendering`](src/rendering/mod.rs) | Bevy presentation systems that project simulation/view state into visuals. |
| [`runtime_census`](src/runtime_census.rs) | Profiling-only presentation census: the views, targets and draw population Ambition hands the renderer. |
| [`screen_effects`](src/screen_effects.rs) | Whole-screen post-processing effects for presentation cameras. |
| [`spirit_field`](src/spirit_field.rs) | The field of a spirit: near each spirit the picture ripples, as air over a fire does, and takes the spirit's colour. |
| [`ui_fonts`](src/ui_fonts.rs) | UI font loading for the presentation layer. |
| [`void_field`](src/void_field.rs) | The field of the void: the picture loses its colour, and a part of its light, near each void thing. |

_21 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
