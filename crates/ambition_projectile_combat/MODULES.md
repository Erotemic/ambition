# `ambition_projectile_combat` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_projectile_combat** — Projectile combat execution above the reusable projectile model.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`allegiance`](src/allegiance.rs) | Projectile combat allegiance that survives the firing body's lifetime. |
| [`clash`](src/clash.rs) | Collect and apply melee and projectile clashes before either damage path. |
| [`feature_contact`](src/feature_contact.rs) | Where along a projectile's travel it first reaches a boss or a breakable. |
| [`intercept`](src/intercept.rs) | INTERCEPTING A SHOT IN FLIGHT, as one named operation. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declaration for the allegiance fixed at shot launch. |
| [`snapshot_impls`](src/snapshot_impls.rs) | Wire format for projectile combat state. |
| [`systems`](src/systems.rs) | The Bevy gameplay system that ticks projectiles, samples motion input, and routes hits through ECS-native feature damage messages. |

_7 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
