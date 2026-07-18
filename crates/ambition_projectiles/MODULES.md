# `ambition_projectiles` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_projectiles** — `ambition_projectiles` — the reusable, content-free projectile MODEL.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`collision_world`](src/collision_world.rs) | The collision world a projectile flies through. |
| [`diagnostics`](src/diagnostics.rs) | Developer-facing logging and HUD summaries for the projectile system. |
| [`enemy`](src/enemy/mod.rs) | Enemy-fired projectiles (pirate volleys etc). |
| [`entity`](src/entity.rs) | Per-projectile ECS entity components (Stage 19 Phase 3c-ii). |
| [`kind`](src/kind.rs) | Named projectile kinds + their authored stat tables (Ambition's basic kit). |
| [`portal_transit`](src/portal_transit.rs) | Projectile portal transit — a small, fast in-flight shot threads a portal aperture (carrying rotated momentum) instead of detonating on the portal wall. |
| [`spawn`](src/spawn.rs) | Cooldown + resource-meter gating for spawning new projectiles. |
| [`spawn_message`](src/spawn_message.rs) | `SpawnProjectile` decouples fire sites from projectile storage. |
| [`spawn_systems`](src/spawn_systems.rs) | Pool-spawn system: turn a queued player-pool [`SpawnProjectile`] message into a live projectile entity. |
| [`state`](src/state.rs) | Per-player projectile controller state: charge machine, motion-input buffer, and tracked unlocks. |
| [`visual`](src/visual.rs) | Projectile visual identity — an **open, content-owned** art registry. |

_11 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
