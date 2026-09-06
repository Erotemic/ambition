# `ambition_geometry` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_geometry** — General geometry and reference-frame primitives.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`combat_volume`](src/combat_volume.rs) | `CombatVolume` — a hit/hurt shape that can be an axis-aligned box, a rotated box (OBB), or a general convex polygon. |
| [`geometry`](src/geometry.rs) | Bevy-native geometry helpers. |
| [`reference_frame`](src/reference_frame.rs) | The gravity-relative reference frame and the transforms between Ambition's three frames. |
| [`swing_shape`](src/swing_shape.rs) | `SwingShape` — the ORIENTED shape of a swing, as presentation needs it. |
| [`volume_shape`](src/volume_shape.rs) | `VolumeShape` — an authored hit/hurt shape in LOCAL space. |

_5 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
