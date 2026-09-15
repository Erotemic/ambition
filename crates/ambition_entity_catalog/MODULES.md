# `ambition_entity_catalog` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_entity_catalog** — Entity-contract + moveset vocabulary — the gameplay-truth schema.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`action_scheme`](src/action_scheme.rs) | Device-free character action vocabulary. |
| [`authoring`](src/authoring.rs) | The primitives a character's move table is written with — shared, because the second character to author one must not begin by copying the first. |
| [`brain_profile_ref`](src/brain_profile_ref.rs) | Naming a shared autonomous-controller policy, in the two forms an authored reference and a resolved identity need to be. |
| [`move_section`](src/move_section.rs) | The move family's own artifact section — fast-iteration packet I2, step 1/3. |
| [`placements`](src/placements.rs) | Pure authored placement schema lowered into runtime behavior by higher layers. |
| [`smash_bolt`](src/smash_bolt.rs) | Authored payload for the steerable bolt technique. |
| [`smash_bomb`](src/smash_bomb.rs) | Authored payload for dropping a bomb. |
| [`smash_capture`](src/smash_capture.rs) | Platform-fighter capture vocabulary: grab, pummel, and throw. |
| [`smash_counter`](src/smash_counter.rs) | Authored payload for a counter stance. |
| [`smash_flyline`](src/smash_flyline.rs) | Authored payload for a flyline recovery. |
| [`smash_homing`](src/smash_homing.rs) | Authored payload for a homing dash. |
| [`smash_limit`](src/smash_limit.rs) | Authored Limit-meter fill policy and the technique that fills the meter directly. |
| [`smash_mark`](src/smash_mark.rs) | Authored payload for a delayed mark applied by a damaging hit. |
| [`smash_mine`](src/smash_mine.rs) | Authored payload for a remotely triggered mine. |
| [`smash_portal`](src/smash_portal.rs) | Authored payload for placing a linked portal pair as a recovery. |
| [`smash_repertoire`](src/smash_repertoire.rs) | Standard Smash action grammar and repertoire bookkeeping. |
| [`smash_ride`](src/smash_ride.rs) | Authored payload for summoning and riding a mount. |
| [`smash_riposte`](src/smash_riposte.rs) | Authored payload for a body-anchored follow-up strike. |
| [`smash_sleep`](src/smash_sleep.rs) | Authored payload for a temporary action lock presented as sleep. |
| [`smash_spring`](src/smash_spring.rs) | Authored payload for placing a temporary spring on the stage. |
| [`smash_teleport`](src/smash_teleport.rs) | Authored payload for teleport movement. |
| [`smash_tether`](src/smash_tether.rs) | Authored payload for reeling a fighter toward a ledge. |
| [`smash_time_dilation`](src/smash_time_dilation.rs) | Authored payload for temporarily slowing a body. |
| [`smash_trapdoor`](src/smash_trapdoor.rs) | Authored payload for entering and leaving the submerged body mode. |
| [`smash_vitality`](src/smash_vitality.rs) | Authored payload for changing the mover's own health. |

_25 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
