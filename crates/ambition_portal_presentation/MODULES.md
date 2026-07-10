# `ambition_portal_presentation` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_portal_presentation** — Default renderer for the headless [`ambition_portal`] mechanic.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`camera_continuity`](src/camera_continuity.rs) | Optional portal camera continuity: presentation-only viewpoint mapping for the camera while a controlled body straddles a portal. |
| [`clip_material`](src/clip_material.rs) | The portal-clip material: a `Material2d` that draws one texture-accurate **piece** of a sprite mid-portal-transit, discarding every fragment behind a world-space clip half-plane. |
| [`effects`](src/effects.rs) | Runtime selection between the compiled-in portal transit **visual effects**, for live A/B comparison and profiling (the view windows cost extra render passes; on constrained targets the host needs to measure that against the bare baseline, in the SAME session). |
| [`gun_visuals`](src/gun_visuals.rs) | Compatibility visuals for Ambition's portal-gun workflow. |
| [`plugin`](src/plugin.rs) | The drop-in presentation plugin + its schedule label. |
| [`view_cones`](src/view_cones.rs) | Through-portal **view windows**: each placed portal shows a slice of the world in front of its partner, set into its host surface — you look "through the portal a little bit" — rendered live by an offscreen capture camera. |
| [`visuals`](src/visuals.rs) | Default portal-seam visuals: portal quads + labels, mid-transit body-piece decomposition, and the disorientation indicator. |

_7 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
