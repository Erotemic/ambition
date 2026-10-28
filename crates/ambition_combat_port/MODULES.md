# `ambition_combat_port` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_combat_port** — The combat domain's extension ports, and the held-item ports of the [`wielded`] module.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`attachments`](src/attachments.rs) | The named points of a body's rig: a jaw, a saddle, a grip, a muzzle, a hand. |
| [`blast`](src/blast.rs) | A blast: damage where a body is, credited to another body. |
| [`contact`](src/contact.rs) | The ports of a contact rule: what a touch on a body does, decided by a module in `body_contact` (`ambition_extension_sdk::phases::BODY_CONTACT`). |
| [`force`](src/force.rs) | A field of force a module holds over a region of its room: a wind, a tide, the low weight of space. |
| [`hold`](src/hold.rs) | A body held by another: seized, carried at a point, pummelled, thrown, released. |
| [`launch`](src/launch.rs) | The axes of an authored hit's launch direction. |
| [`ledge`](src/ledge.rs) | A body and the stage's ledges: the facts a move that reaches for a ledge reads, and the line the player sees while it does. |
| [`lightning`](src/lightning.rs) | Lightning a module shows, not one it strikes with: a bolt between two ends, and an aura on its own body. |
| [`module_entity`](src/module_entity.rs) | Module-owned entities: a module asks the world for an entity of a kind it names (a turret, a well), and the world ticks each such entity through the module, until the entity's lifetime ends. |
| [`motion`](src/motion.rs) | The body-motion ports of a wielded item: a body moves itself or sets its velocity, arms the shared movement cooldown, strikes where it is, and shows an effect or a hit mark. |
| [`ray`](src/ray.rs) | A ray: a strike along a line. |
| [`reaction`](src/reaction.rs) | The ports of a reaction to a hit: what a body does about how this tick's hits came out, decided by a module in `hit_reaction` (`ambition_extension_sdk::phases::HIT_REACTION`). |
| [`ride`](src/ride.rs) | A body that summons something to ride, and the summon itself: the verbs a move that calls a mount needs, and the facts it reads to decide when the ride ends. |
| [`riding`](src/riding.rs) | A volume that rides its owner, and a burst of particles: what a conducted boss swings and shows. |
| [`slow`](src/slow.rs) | A slow on the bodies in a box: honey, glue, tar. |
| [`special`](src/special.rs) | The ports of a fighter's special: what a special move does when its fighter presses it, decided by a module in `technique_execution` (`ambition_extension_sdk::phases::TECHNIQUE_EXECUTION`). |
| [`status`](src/status.rs) | A body's statuses: state a module puts on a body and ticks there. |
| [`targets`](src/targets.rs) | What a body may aim at: the facts a module's targeting reads. |
| [`think`](src/think.rs) | The ports of a custom brain: a body whose brain is a `CharacterBrain::Custom` key that a module binds decides through `actor_think` (`ambition_extension_sdk::phases::ACTOR_THINK`). |
| [`wielded`](src/wielded.rs) | The held-item domain's extension ports: a body uses the item it holds, pays for it from its resource bank, and is heard doing it. |

_20 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
