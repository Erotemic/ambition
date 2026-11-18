# `ambition_projectiles` — module map

<!-- BEGIN generated module map (scripts/measure/modules_md.py) -->

**ambition_projectiles** — Physical projectile vocabulary, construction, and flight services.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`birth`](src/birth.rs) | Physical projectile construction and material admission. |
| [`collision_world`](src/collision_world.rs) | The room geometry and aperture snapshot used by projectile contacts. |
| [`content_schema`](src/content_schema.rs) | The `projectile_visuals` schema: a game's named projectile looks, as data in its pack. |
| [`diagnostics`](src/diagnostics.rs) | Developer-facing logging and HUD summaries for the projectile system. |
| [`entity`](src/entity.rs) | Per-projectile ECS entity components (Stage 19 Phase 3c-ii). |
| [`kind`](src/kind.rs) | Named projectile kinds + their authored stat tables (Ambition's basic kit). |
| [`ownership`](src/ownership.rs) | Current projectile credit, separate from creation provenance and live links. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declaration owned by `ambition_projectiles`. |
| [`snapshot_impls`](src/snapshot_impls.rs) | `SnapshotState` for this crate's own types — the rollback wire format. |
| [`spawn`](src/spawn.rs) | Cooldown + resource-meter gating for spawning new projectiles. |
| [`state`](src/state.rs) | Per-player projectile controller state: charge machine, motion-input buffer, and tracked unlocks. |
| [`visual`](src/visual.rs) | Projectile visual identity — an open, content-owned art registry. |

_12 crate-root modules. Regenerate: `python scripts/measure/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
