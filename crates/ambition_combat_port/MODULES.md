# `ambition_combat_port` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_combat_port** — The combat domain's extension ports, and the held-item ports of the [`wielded`] module.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`module_entity`](src/module_entity.rs) | Module-owned entities: a module asks the world for an entity of a kind it names (a turret, a well), and the world ticks each such entity through the module, until the entity's lifetime ends. |
| [`riding`](src/riding.rs) | A volume that rides its owner, and a burst of particles: what a conducted boss swings and shows. |
| [`wielded`](src/wielded.rs) | The held-item domain's extension ports: a body uses the item it holds, pays for it from its resource bank, and is heard doing it. |

_3 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
