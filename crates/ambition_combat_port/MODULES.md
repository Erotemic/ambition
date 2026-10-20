# `ambition_combat_port` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_combat_port** — The combat domain's extension ports, and the held-item ports of the [`wielded`] module.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`attachments`](src/attachments.rs) | The named points of a body's rig: a jaw, a saddle, a grip, a muzzle, a hand. |
| [`contact`](src/contact.rs) | The ports of a contact rule: what a touch on a body does, decided by a module in `body_contact` (`ambition_extension_sdk::phases::BODY_CONTACT`). |
| [`hold`](src/hold.rs) | A body held by another: seized, carried at a point, pummelled, thrown, released. |
| [`lightning`](src/lightning.rs) | Lightning a module shows, not one it strikes with: a bolt between two ends, and an aura on its own body. |
| [`module_entity`](src/module_entity.rs) | Module-owned entities: a module asks the world for an entity of a kind it names (a turret, a well), and the world ticks each such entity through the module, until the entity's lifetime ends. |
| [`motion`](src/motion.rs) | The body-motion ports of a wielded item: a body moves itself or sets its velocity, arms the shared movement cooldown, strikes where it is, and shows an effect or a hit mark. |
| [`ray`](src/ray.rs) | A ray: a strike along a line. |
| [`riding`](src/riding.rs) | A volume that rides its owner, and a burst of particles: what a conducted boss swings and shows. |
| [`slow`](src/slow.rs) | A slow on the bodies in a box: honey, glue, tar. |
| [`special`](src/special.rs) | The ports of a fighter's special: what a special move does when its fighter presses it, decided by a module in `technique_execution` (`ambition_extension_sdk::phases::TECHNIQUE_EXECUTION`). |
| [`status`](src/status.rs) | A body's statuses: state a module puts on a body and ticks there. |
| [`targets`](src/targets.rs) | What a body may aim at: the facts a module's targeting reads. |
| [`think`](src/think.rs) | The ports of a custom brain: a body whose brain is a `CharacterBrain::Custom` key that a module binds decides through `actor_think` (`ambition_extension_sdk::phases::ACTOR_THINK`). |
| [`wielded`](src/wielded.rs) | The held-item domain's extension ports: a body uses the item it holds, pays for it from its resource bank, and is heard doing it. |

_14 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
