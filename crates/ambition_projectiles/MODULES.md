# `ambition_projectiles` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_projectiles** — Reusable, content-free projectile vocabulary and materialization.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`collision_world`](src/collision_world.rs) | The collision world a projectile flies through. |
| [`diagnostics`](src/diagnostics.rs) | Developer-facing logging and HUD summaries for the projectile system. |
| [`entity`](src/entity.rs) | Per-projectile ECS entity components (Stage 19 Phase 3c-ii). |
| [`kind`](src/kind.rs) | Named projectile kinds + their authored stat tables (Ambition's basic kit). |
| [`materialize`](src/materialize.rs) | Projectile request → ECS materialization. |
| [`portal_transit`](src/portal_transit.rs) | The core ([`try_projectile_portal_transit`]) is pure + deterministic — no Bevy, no RNG — so the transit geometry (does it cross? where does it pop out? which way does momentum rotate?) is headless-testable. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declaration owned by `ambition_projectiles`. |
| [`snapshot_impls`](src/snapshot_impls.rs) | `SnapshotState` for this crate's own types — the rollback wire format. |
| [`spawn`](src/spawn.rs) | Cooldown + resource-meter gating for spawning new projectiles. |
| [`spawn_request`](src/spawn_request.rs) | Authoritative projectile-spawn request vocabulary. |
| [`state`](src/state.rs) | Per-player projectile controller state: charge machine, motion-input buffer, and tracked unlocks. |
| [`visual`](src/visual.rs) | Projectile visual identity — an open, content-owned art registry. |

_12 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
