# `ambition_characters` — module map

<!-- BEGIN generated module map (scripts/modules_md.py) -->

**ambition_characters** — Content-free character identity, control, behavior, and authoring vocabulary.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`action_scheme`](src/action_scheme.rs) | Runtime action scheme — deriving a body's [`ActionSchemeContract`] from the SAME authorities that gate its behavior, and carrying it as an ECS component. |
| [`actor`](src/actor/mod.rs) | Reusable, content-free actor vocabulary: identity + the control contract. |
| [`binding_namespaces`](src/binding_namespaces.rs) | THE NAMESPACES CHARACTER PREPARATION RESOLVES AGAINST — the vocabulary every cross-layer reference a character makes is checked in. |
| [`boss_encounter`](src/boss_encounter.rs) | Boss phase schema and trigger-driven progression. |
| [`brain`](src/brain/mod.rs) | Universal brain interface. |
| [`control`](src/control.rs) | **WHO IS DRIVING, and what they pressed.** The per-seat control vocabulary: seat identity, the tables keyed by it, and the component that says which body a seat drives. |
| [`equipment`](src/equipment.rs) | Content-free equipment rules. |
| [`load_demand`](src/load_demand.rs) | What characters a composition has asked to have realized. |
| [`moveset_content_schema`](src/moveset_content_schema.rs) | The `moveset` authored-content schema — fast-iteration I2, step 4. |
| [`moveset_prefabs`](src/moveset_prefabs.rs) | Move authoring — the build-time half of the Smash model: the functions that turn authored specs (`MeleeActionSpec`/`RangedActionSpec`), tunable params (`Simple{Melee,Ranged,Charge}Params`), and the `MovePrefabRegistry` into `MoveSpec`s, plus `build_actor_moveset` which assembles an actor's full `MovesetContract` from its catalog + worn equipment. |
| [`perception`](src/perception.rs) | Controller-neutral per-body perception. |
| [`prepared`](src/prepared.rs) | Character registration and preparation. |
| [`prepared_fixtures`](src/prepared_fixtures.rs) | Fixture builders shared by preparation's own tests and the registration tests one crate up. |
| [`rollback_registration`](src/rollback_registration.rs) | Rollback declaration owned by `ambition_characters`. |
| [`smash_fighter`](src/smash_fighter/mod.rs) | Character-owned authored `smash_fighter` facet. |
| [`smash_hold_state`](src/smash_hold_state.rs) | `SmashHoldState` — the platform-fighter RULES of a hold, as runtime state. |
| [`snapshot_impls`](src/snapshot_impls.rs) | `SnapshotState` for this crate's own types — the rollback wire format. |
| [`technique`](src/technique.rs) | THE AUTHORED SCHEMAS OF ENGINE TECHNIQUES — the params an `on_hit` effect carries, and nothing that executes one. |

_18 crate-root modules. Regenerate: `python scripts/modules_md.py --write`._

<!-- END generated module map -->

## Notes

_Hand-written notes live here and survive regeneration: the crate's authoritative state, its seams, and anything the module headers cannot say._
