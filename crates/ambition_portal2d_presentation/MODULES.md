# `ambition_portal2d_presentation` — module map

<!-- BEGIN generated module map (scripts/measure/modules_md.py) -->

**ambition_portal2d_presentation** — Default renderer for the headless [`ambition_portal2d`] mechanic.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`camera_continuity`](src/camera_continuity.rs) | Optional portal camera continuity: presentation-only viewpoint mapping for the camera while a controlled body straddles a portal. |
| [`carry`](src/carry.rs) | A portal pair carries what is drawn into its entry out of its exit: every kind of drawable, by one road (`docs/planning/engine/portals-a-body-in-two-places.md`, "One road for every drawable"). |
| [`clip_material`](src/clip_material.rs) | Textured portal pieces with complete mesh clipping. |
| [`effects`](src/effects.rs) | Runtime selection between the compiled-in portal transit visual effects, for live A/B comparison and profiling (the view windows cost extra render passes; on constrained targets the host needs to measure that against the bare baseline, in the SAME session). |
| [`glow`](src/glow.rs) | The portal's line of light: where the opening of an aperture is. |
| [`limb_seams`](src/limb_seams.rs) | Where a limb that goes through a portal is cut, and the lines of light that are drawn on those cuts. |
| [`plugin`](src/plugin.rs) | The drop-in presentation plugin + its schedule label. |
| [`shot_visuals`](src/shot_visuals.rs) | Portal shots in flight. |
| [`view_cones`](src/view_cones.rs) | Live through-portal view windows. |
| [`viewpoints`](src/viewpoints.rs) | Host viewpoints shared by portal presentation passes. |
| [`visuals`](src/visuals.rs) | Default portal-seam visuals: portal labels and the disorientation indicator. |

_11 crate-root modules. Regenerate: `python scripts/measure/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
