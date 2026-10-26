# `ambition_encounter_features` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_encounter_features** — Generic, reusable enemy-WAVE / arena-lockdown system (data-driven, not scripted) — distinct from `ambition_boss_encounter`, which is one specific scripted boss fight with hand-authored phases.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`authored_switch_commands`](src/authored_switch_commands.rs) | Runs commands authored directly on `Switch` entities. |
| [`conditions`](src/conditions.rs) | Authored ENCOUNTER conditions — "what became of this arena?" |
| [`loading`](src/loading.rs) | Room → `EncounterSpec` loader plus the content-installed wave book. |
| [`lock_walls`](src/lock_walls.rs) | Lock-wall contribution: the solid blocks that seal an arena's exits while an encounter is in flight. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declarations for derived world state. |
| [`spawn_service`](src/spawn_service.rs) | Build the bodies requested by encounter waves. |
| [`switch_index`](src/switch_index.rs) | The switch INDEX rebuild, which stayed behind when the switch types left. |
| [`systems`](src/systems.rs) | The Bevy adapters around the generic encounter lifecycle (E8/E9). |

_8 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
