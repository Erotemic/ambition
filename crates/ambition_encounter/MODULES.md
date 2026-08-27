# `ambition_encounter` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_encounter** — Generic encounter orchestration: the ONE lifecycle authority, command ingress, objectives, participants, timeline, and wave policy.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`authored_commands`](src/authored_commands.rs) | Authored `encounter.signal` command. |
| [`content_schema`](src/content_schema.rs) | The `encounter_waves` authored-content schema, owned by this capability. |
| [`entity`](src/entity.rs) | The encounter as a first-class ENTITY. |
| [`events`](src/events.rs) | `EncounterEvent` — the output stream of the encounter lifecycle reducer and the wave director. |
| [`lifecycle`](src/lifecycle.rs) | Generic encounter lifecycle authority. |
| [`music`](src/music.rs) | The single encounter→audio music-intent stream. |
| [`objective`](src/objective.rs) | Generic encounter OBJECTIVES (§5): a small predicate vocabulary over participants, elapsed time, and received signals. |
| [`participants`](src/participants.rs) | Generic encounter PARTICIPANTS (§3): membership as relations, not boss-specific `Vec<Entity>`. |
| [`registry`](src/registry.rs) | `EncounterRegistry` resource: the `id -> Entity` INDEX into the live encounter entities (E1 — the live state lives on the entity's [`EncounterState`](crate::EncounterState) component, not here). |
| [`rewards`](src/rewards.rs) | Encounter reward-chest helpers: `encounter_reward_looted_flag` (the per-encounter save-flag id that remembers a chest was opened across save/load) and `encounter_reward_chest_pos` (where the `EncounterSpec`'s reward chest spawns — centered on the trigger, resting on its floor). |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declaration owned by `ambition_encounter`. |
| [`snapshot_impls`](src/snapshot_impls.rs) | `SnapshotState` for this crate's own types — the rollback wire format. |
| [`spec`](src/spec.rs) | Authored encounter data types (serde RON). |
| [`staging`](src/staging.rs) | Authored STAGING policy (E12): the presentation effects an encounter wants while it is in flight, as components the generic consumers derive from the LIFECYCLE — never from what kind of encounter it is. |
| [`switches`](src/switches.rs) | Switch-arming gate for encounters. |
| [`timeline`](src/timeline.rs) | Generic encounter TIMELINE (§6): ordered beats `{ when: Trigger, then: [Effect] }` that advance as triggers fire. |
| [`waves`](src/waves.rs) | Wave POLICY: the spawn-cadence director for wave/arena encounters. |

_17 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
