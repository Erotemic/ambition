# `ambition_characters` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_characters** — The actor BEHAVIOR + identity layer — the "minds and cast" of the workspace.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`action_scheme`](src/action_scheme.rs) | Runtime action scheme — deriving a body's [`ActionSchemeContract`] from the SAME authorities that gate its behavior, and carrying it as an ECS component. |
| [`actor`](src/actor/mod.rs) | Reusable, content-free actor vocabulary: identity + the control contract. |
| [`binding_namespaces`](src/binding_namespaces.rs) | **THE NAMESPACES CHARACTER PREPARATION RESOLVES AGAINST** — the vocabulary every cross-layer reference a character makes is checked in. |
| [`boss_encounter`](src/boss_encounter.rs) | Boss encounter state machine. |
| [`brain`](src/brain/mod.rs) | Universal brain interface. |
| [`equipment`](src/equipment.rs) | A3 — equipment→params. |
| [`moveset_authoring`](src/moveset_authoring.rs) | **The primitives a character's move table is written with** — shared, because the second character to author one must not begin by copying the first. |
| [`moveset_prefabs`](src/moveset_prefabs.rs) | **Move authoring** — the build-time half of the Smash model: the functions that turn authored specs (`MeleeActionSpec`/`RangedActionSpec`), tunable params (`Simple{Melee,Ranged,Charge}Params`), and the `MovePrefabRegistry` into `MoveSpec`s, plus `build_actor_moveset` which assembles an actor's full `MovesetContract` from its catalog + worn equipment. |
| [`perception`](src/perception.rs) | `WorldView` + `WorldMemory` — the **world-out** port (architecture roadmap S4). |
| [`prepared`](src/prepared.rs) | **One registration per character.** (§4.1, §4.6, §5) |
| [`prepared_fixtures`](src/prepared_fixtures.rs) | Fixture builders shared by preparation's own tests and the registration tests one crate up. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declaration owned by `ambition_characters`. |
| [`smash_capture`](src/smash_capture.rs) | **THE CAPTURE VOCABULARY — grab, pummel, throw, authored once.** |
| [`smash_fighter`](src/smash_fighter/mod.rs) | **THE FIRST CHARACTER-OWNED `smash.fighter` FACET — values authored as content, prepared into runtime fighter data.** |
| [`smash_repertoire`](src/smash_repertoire.rs) | **THE STANDARD SMASH REPERTOIRE — the vocabulary and the bookkeeping, once.** |
| [`snapshot_impls`](src/snapshot_impls.rs) | `SnapshotState` for this crate's own types — the rollback wire format. |
| [`technique`](src/technique.rs) | **THE AUTHORED SCHEMAS OF ENGINE TECHNIQUES** — the params an `on_hit` effect carries, and nothing that executes one. |

_17 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
