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
| [`smash_bolt`](src/smash_bolt.rs) | A bolt the caster steers with the same stick they walk with. |
| [`smash_bomb`](src/smash_bomb.rs) | Put a live bomb on the stage: the authored vocabulary. |
| [`smash_capture`](src/smash_capture.rs) | Platform-fighter capture vocabulary: grab, pummel, and throw. |
| [`smash_counter`](src/smash_counter.rs) | The counter stance: the authored vocabulary for "if you hit me here, this happens". |
| [`smash_flyline`](src/smash_flyline.rs) | Being lifted out of the scene on a wire: the authored vocabulary. |
| [`smash_homing`](src/smash_homing.rs) | Carry the fighter at whoever they were pointing at: the authored vocabulary. |
| [`smash_limit`](src/smash_limit.rs) | The Limit meter: what fills a fighter's meter, authored rather than assumed. |
| [`smash_mark`](src/smash_mark.rs) | A strike that leaves a delayed mark on the body it hits: the authored vocabulary. |
| [`smash_mine`](src/smash_mine.rs) | Place a mine the placer can set off from anywhere: the authored vocabulary. |
| [`smash_portal`](src/smash_portal.rs) | The portal recovery: the authored vocabulary for "open a way up". |
| [`smash_repertoire`](src/smash_repertoire.rs) | Standard Smash action grammar and repertoire bookkeeping. |
| [`smash_ride`](src/smash_ride.rs) | Summon-a-mount-and-ride: the authored vocabulary. |
| [`smash_riposte`](src/smash_riposte.rs) | Answer a parry with the blade: the authored vocabulary. |
| [`smash_sleep`](src/smash_sleep.rs) | Putting a body to sleep: the authored vocabulary. |
| [`smash_spring`](src/smash_spring.rs) | Leave a plate on the stage that throws whoever steps on it. |
| [`smash_teleport`](src/smash_teleport.rs) | Teleport-as-a-recovery: the authored vocabulary. |
| [`smash_tether`](src/smash_tether.rs) | Reel the fighter to a ledge she threw a tether at: the authored vocabulary. |
| [`smash_time_dilation`](src/smash_time_dilation.rs) | The time-dilation technique: "for a moment, you are slower than the world." |
| [`smash_trapdoor`](src/smash_trapdoor.rs) | Going under the stage and coming back: the authored vocabulary. |
| [`smash_vitality`](src/smash_vitality.rs) | A move that changes its own mover's health: the authored vocabulary. |

_25 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
