# `ambition_sim_view` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_sim_view** — **[the observation boundary]** — the `SimView` read-model (E4).

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`anim_index`](src/anim_index.rs) | The per-actor POSE index + the per-boss FRAME index (E4 slices 3, 7, 19): id-keyed read-models rebuilt once per sim tick; presentation animates from these snapshots and never borrows the live clusters. |
| [`camera_snapshot`](src/camera_snapshot.rs) | Pure 2D camera-follow snapshot policy. |
| [`dialog_view`](src/dialog_view.rs) | `DialogView` — the dialogue overlay's per-frame read-model (recon C3). |
| [`facts`](src/facts.rs) | The observation-boundary staging ground (E4): small sim-resolved view resources presentation consumes INSTEAD of querying live sim components. |
| [`pose_view`](src/pose_view.rs) | Per-body presentation POSE read-model for player-bodied entities (E4). |
| [`view_index`](src/view_index.rs) | `FeatureViewIndex` resource and the per-frame rebuild pass. |

_6 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
