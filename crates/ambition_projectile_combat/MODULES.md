# `ambition_projectile_combat` — module map

<!-- BEGIN generated module map (scripts/measure/modules_md.py) -->

**ambition_projectile_combat** — Projectile combat execution above the reusable projectile model.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`clash`](src/clash.rs) | Collect and apply melee and projectile clashes before either damage path. |
| [`feature_contact`](src/feature_contact.rs) | Admitted hurt-material contacts for bosses and breakables. |
| [`intercept`](src/intercept.rs) | Prepare and commit one projectile interception. |
| [`systems`](src/systems.rs) | The Bevy gameplay system that ticks projectiles, samples motion input, and routes hits through ECS-native feature damage messages. |

_4 crate-root modules. Regenerate: `python scripts/measure/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
