---
id: engine-mental-model
aliases: []
status: current
authority: durable-concept
last_verified: 2026-07-18
related_docs:
  - docs/planning/vision.md
  - docs/planning/engine/architecture.md
  - docs/concepts/content-and-provider-boundaries.md
  - docs/concepts/sim-presentation-seam.md
  - docs/adr/0025-character-actions-input-ownership.md
---

# Engine mental model

Read this after `README.md`, `AGENTS.md`, and `.agent/README.md`. It is the
smallest durable model of the repository: enough to decide which layer should
own a change without memorizing the current crate tree.

## The product

Ambition is two things developed together:

1. a reusable, composable, Bevy-native 2D platformer engine; and
2. Ambition, the first game/provider built on that engine.

The design oracle is:

> Could another platformer be built by adding a provider/content crate and a
> thin host without editing reusable engine crates?

The demo games make that question executable. Game-specific names, rosters,
worlds, art, dialogue, music, and rules belong above the reusable engine.

## The seven-layer picture

The exact crate list will continue to change. These responsibilities should not.

| Layer | Responsibility | Typical current owners |
|---|---|---|
| Foundations | Pure math, geometry, movement kernels, stable IDs, and content-free data contracts | `ambition_engine_core`, `ambition_entity_catalog` |
| Shared platformer vocabulary | Lifecycle scopes, scheduling sets, gravity/orientation, world and interaction primitives | `ambition_platformer_primitives`, `ambition_world`, focused domain crates |
| Domain services | Characters/brains, combat, input, dialogue, encounters, persistence, audio, portals, items, projectiles | one focused crate per domain |
| Simulation heart | Assemble live actor bodies and world mechanics through one execution path | currently `ambition_actors` plus domain crates |
| Observation and presentation | Publish stable read models; render, animate, play audio, and show UI without owning simulation truth | `ambition_sim_view`, `ambition_render`, presentation crates |
| Runtime, provider, and host | Order headless-safe plugins, prepare/activate providers, and add devices/windows/platform policy | `ambition_runtime`, `ambition_platformer_provider`, `ambition_host`, `ambition_sim_harness` |
| Games/providers | Own named worlds, characters, art, audio, quests, dialogue, encounters, and product policy | `game/ambition_content`, demo providers, thin app crates |

Use `python scripts/agent_query.py crate <name>` for the current package map.
Do not turn this page into a manually maintained list of every crate.

## One body, one path

Player, enemy, boss, NPC, mount, possessed body, and RL-controlled body are not
separate engine species. They are one actor/body model with different:

- identity and authored capabilities;
- equipment and movement/combat configuration;
- controller or brain;
- perception and memory;
- current control authority.

Before adding a player-only or enemy-only system, ask whether the other
controller kind already performs the same operation elsewhere. If so, route
both through one seam and delete the duplicate. Similar behavior on parallel
paths is not unification.

The orchestrating Bevy systems may remain separate where scheduling or host
policy differs. The body/move/combat implementation they call must be shared.

## Control and action flow

The stable control shape is:

```text
physical device / touch / RL / brain
    -> semantic device actions
    -> control authority selects a subject
    -> actor-local control intent
    -> ActorActionScheme describes what each slot means for this body
    -> shared slot resolver gates/reroutes the intent
    -> movement kernel / MovePlayback / interaction systems mutate simulation
    -> ControlPrompt publishes the same resolved meaning to UI and adapters
```

Important consequences:

- Devices emit semantic slots, not character-specific moves.
- A character's live authorities derive its action scheme; the scheme is not a
  second authored source of truth.
- Gameplay and prompts use the same resolver. A button must not advertise an
  action the body cannot perform.
- `InputState::control_dt` is the input-side precision-clock affordance. It does
  not justify a second player simulation pipeline.
- Brains and humans ultimately drive the same actor-local action/body seams.

See [`input-and-game-modes.md`](input-and-game-modes.md) and
[`../systems/input-control-and-ui.md`](../systems/input-control-and-ui.md).

## World, content, and session flow

The durable world/content flow is:

```text
LDtk or another authoring backend
    -> typed backend adapter
    -> backend-neutral authored world records
    -> validation and lowering registries
    -> provider-owned immutable content fragments
    -> prepared content / construction plan
    -> one transaction commits a session or room
    -> simulation state
    -> observation read models
    -> presentation
```

Rules:

- LDtk owns Ambition's spatial authoring today; reusable simulation does not
  depend on LDtk JSON shapes.
- Import/deserialization, validation, lowering, construction, and commit are
  different phases. Do not mutate the live world during preflight.
- The old room/session remains authoritative until the replacement can commit.
- Prepared content is immutable evidence, not live gameplay authority.
- Live authority belongs to the exact session scope and its entities/resources.
- Provider-owned IDs are stable content identity. Bevy `Entity` values are
  allocator handles and must not become persisted identity.

See [`content-and-provider-boundaries.md`](content-and-provider-boundaries.md)
and [`../systems/ldtk-world-composition.md`](../systems/ldtk-world-composition.md).

## Simulation and presentation

Simulation owns anything that can change an outcome. Presentation consumes
messages and read models and may not mutate authoritative simulation to make a
visual effect convenient.

A useful test is:

> Can the same provider run headlessly and reach the same authoritative state
> without creating cameras, sprites, audio devices, or menus?

If not, either presentation owns too much or the headless composition is
incomplete. See [`sim-presentation-seam.md`](sim-presentation-seam.md).

## Determinism and reconstruction

The engine is designed for headless tests, replay, rollback-ready snapshots,
local multiplayer, and forward-model AI. Therefore:

- authoritative decisions cannot depend on wall-clock duration;
- iteration order that affects outcomes must be explicit and stable;
- derived read models are rebuilt rather than persisted as competing truth;
- reset, room replacement, provider switching, and snapshot restore must use
  the same canonical construction/lowering seams;
- process-global mirrors of session state are suspicious by default.

Bit-identical replay is a canary, not a command to preserve bad pre-release
behavior. Preserve invariants and intentional semantics, not accidental output.

## How to place a change

Ask these questions in order:

1. Is this named game content or reusable capability?
2. Is it authored data, live simulation truth, a derived read model, or a side effect?
3. Which single domain owns mutation authority?
4. Does another controller/body/provider already do this on a different path?
5. Can it run and be tested headlessly?
6. Can another game provide a different implementation without editing core?
7. Can reset/restore/provider switch reconstruct it exactly?

Then localize the current owner:

```bash
python scripts/agent_query.py "<task words>"
python scripts/agent_query.py docs "<invariant>"
python scripts/agent_query.py ecs "<resource or system>"
python scripts/agent_query.py tests "<behavior>"
python scripts/agent_query.py crate <likely-owner>
```

Generated indexes locate evidence. Source decides current fact; active planning
and ADRs decide intended direction.
