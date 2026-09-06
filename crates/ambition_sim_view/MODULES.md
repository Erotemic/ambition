# `ambition_sim_view` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_sim_view** — Plain-data observation boundary over simulation state.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`affordances`](src/affordances/mod.rs) | What the interact button would do right now. |
| [`anim_index`](src/anim_index.rs) | The per-actor POSE index + the per-boss FRAME index (E4 slices 3, 7, 19): id-keyed read-models rebuilt once per sim tick; presentation animates from these snapshots and never borrows the live clusters. |
| [`attack_vfx_view`](src/attack_vfx_view.rs) | Resolved attack-art facts for presentation. |
| [`camera_snapshot`](src/camera_snapshot.rs) | Pure 2D camera-follow snapshot policy. |
| [`combat_geometry_view`](src/combat_geometry_view.rs) | Body-generic combat geometry for observers. |
| [`control_prompt`](src/control_prompt.rs) | `ControlPrompt` — the read-model of "what does each on-screen control do right now, and what is it called," for whatever currently owns input. |
| [`defense_view`](src/defense_view.rs) | Presentation-facing semantic causes of body untouchability. |
| [`dialog_view`](src/dialog_view.rs) | `DialogView` — the dialogue overlay's per-frame read-model (recon C3). |
| [`facts`](src/facts.rs) | The observation-boundary staging ground (E4): small sim-resolved view resources presentation consumes INSTEAD of querying live sim components. |
| [`local_view`](src/local_view.rs) | Per-observer presentation state for one local simulation view. |
| [`pose_view`](src/pose_view.rs) | Per-body presentation POSE read-model for player-bodied entities (E4). |
| [`presented_pose`](src/presented_pose.rs) | Presentation-time body poses sampled from tick read-models. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declaration owned by `ambition_sim_view`. |
| [`view_index`](src/view_index.rs) | `FeatureViewIndex` resource and the per-frame rebuild pass. |

_14 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
