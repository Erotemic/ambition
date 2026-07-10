# `ambition_encounter` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_encounter** — Generic encounter wave/lockdown vocabulary and headless state machine.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`events`](src/events.rs) | `EncounterEvent` — the output stream of the encounter state machine (`state.rs`). |
| [`music`](src/music.rs) | Encounter→audio music request resources. |
| [`registry`](src/registry.rs) | `EncounterRegistry` resource: the multi-encounter holder keyed by id (matching LDtk `EncounterTrigger.id`), so the sandbox runs several encounters at once. |
| [`rewards`](src/rewards.rs) | Encounter reward-chest helpers: `encounter_reward_looted_flag` (the per-encounter save-flag id that remembers a chest was opened across save/load) and `encounter_reward_chest_pos` (where the `EncounterSpec`'s reward chest spawns — centered on the trigger, resting on its floor). |
| [`spec`](src/spec.rs) | Authored encounter data types (serde RON). |
| [`state`](src/state.rs) | The headless encounter state machine: `EncounterPhase` (Inactive→Starting→Active→Cleared/Failed), the per-run `EncounterRun` (pending/alive/elapsed), and the `EncounterState` resource. |

_6 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
