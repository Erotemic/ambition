# Unified actors

The heart of the engine. This consolidates the prior fighter-unification,
NPC/enemy-unification, non-player-centric, locomotion-split, and universal-brain
plans into one statement of where actor control is going and how we get there.

---

## The thesis

**Every actor — the player included — is one body.** A body is:

- **kinematics** (position / velocity / size / facing, frame-agnostic),
- a set of **composable ability limbs** (run, jump, dash, wall-cling, ledge-grab,
  dodge, blink, flight, shield, …), each reading and writing only its own state,
- a **capability mask** that selects which limbs are live,

driven by a **Controller** through one input seam, and observed through one headless
**world-out** view.

> There is no notion of an NPC. Everyone is an actor controlled by a brain. The
> *only* thing that makes an enemy an enemy is that **it wants to kill you**.

> Entity identity chooses a **brain backend**, not a bespoke simulation loop.

"Player", "Enemy", "Boss", "NPC" are **data** — a `(Controller, capabilities,
faction)` tuple — not types, not code paths, not branches in the simulation. The
player's movement is the good, polish-ready base; **enemies rise to it**. We never
drag the player down onto a simpler path.

When this is done, possession ("play as the goblin"), boss-drops ("field the
player-robot as a boss"), and an RL policy ("drop in `Brain::RlPolicy`") cost almost
nothing, because they are the same body with a different controller.

## Invariants (non-negotiable)

1. **One input seam.** Every controller emits an `ActorControlFrame`. `Human` /
   `Brain` / `Remote` / `RlPolicy` are mutually substitutable on any body.
2. **The body enforces; the controller only attempts.** Fire-rate, cooldown, stun,
   traction, *which abilities exist* — all live on the body. A controller spamming
   an intent is harmless; the body is the floor. (If the only reason a body doesn't
   stream attacks is that the controller declines to, the design has failed — a
   human could spam it.)
3. **Degenerate inputs are the world's problem, not the controller's.** The action
   space stays fully open (so an RL agent can probe it); body cooldowns, the arena,
   and counterplay — not a hobbled controller — make lines uninteresting. Restraint
   is *policy* (hand-coded now, learnable later), never enforcement.
4. **Perception is a headless viewport.** Each body sees a world-space region around
   it — the AI analogue of the player's screen — computed from gameplay state, with
   **zero rendering dependency**. The camera may be framed to match a body's
   viewport; the camera consumes perception, never the reverse.
5. **Bodies remember what they've seen.** A controller can pursue a target that left
   its viewport (decaying last-known position), exactly like a human.
6. **Hostility is relational.** Who-fights-whom is a `FactionRelations` matrix, not a
   player-vs-enemy bipartite split. An Enemy can target and damage a Boss while a
   Neutral observer is spared; "ignore the player" / "hostile to faction X" are
   expressible.
7. **Frame-agnostic throughout (relativity).** Perception, motor, and abilities live
   in the acceleration frame — correct under rotated (C4) gravity and through
   portals. No code assumes `-y` is up.
8. **Correct behavior is emergent from structure, not preserved by per-tick hacks.**

## Architecture

### The two ports

Every body exposes exactly two ports; every controller plugs into the same two:

- **intent-in** — the body resolves an `ActorControlFrame` into effects, enforcing
  its own physics and returning per-intent *accepted | blocked* feedback. Movement is
  just another intent.
- **world-out** — the body produces a controller-neutral, **headless** `WorldView`
  (what it can perceive) plus a `WorldMemory` (what it has seen).

The brain pipeline is one path regardless of backend:
**`Brain` → `ActionSet` / `ActorControl` → `ActorActionMessage`** (action consumers —
melee, projectile, boss specials — read the message channel). Player movement rides
`ActorControl`, *not* a player-only seam. Prefer enum dispatch over `dyn Brain`. Never
leak ECS/world access into a brain's tick — that keeps brains pure, replayable, and
RL-droppable.

**Brain backends** (a reference vocabulary, not a checklist of done-ness):
`Player`, `Remote`, `RlPolicy`, `StateMachine`, `BossPattern`, `Scripted`. Add the
inference/training behind `RlPolicy` only when a concrete consumer lands — no
speculative FFI.

### The motion vocabulary — two fields, one rule

The body-local intent and the choreography command are **different control
modalities, not different actor types**:

- **`locomotion: Vec2`** — normalized body-local intent (`|·| ≤ 1`), a throttle of
  *what this body can do*. Every self-propelled actor (player, grounded AI, NPC) uses
  it; the integrator resolves velocity uniformly as `locomotion * max_run_speed`,
  with **no per-actor-type branch**. Per-spawn speed jitter is the brain *choosing to
  throttle* (intent), not a varying capability — a body's `max_run_speed` is fixed
  ("what it can do").
- **`velocity_target` (world-space px/s)** — an exact velocity command for the
  free-mover / choreography modality: boss patterns that snap to a scripted path, AI
  flyers that steer 2D directly. The floating integrator reads this; grounded reads
  `locomotion`. Each consumer picks the field for its mode.

This dual meaning is **essential complexity, not debt** — do not "fix" it by
decomposing velocity back into intent per tick.

### One spine, one floating mover

- Grounded bodies run the **shared spine** `integrate_normal_spine` (+
  `NormalSpineCtx::bare` for bodies without ability limbs): gravity-relative gravity,
  run, fall-cap, fast-fall/glide gates — pay-for-use.
- Aerial bodies run the shared **`step_floating_body`** (`accel: None` = snap to a
  pattern).
- **Platform riding is emergent** — blocks carry velocity; there is no rider list.
- Movement physics is **per-body data** (`BodyMovementTuning`, composed
  hierarchically per archetype), not hardcoded constants.

### Relational hostility, in-place provoke

Hostility is `FactionRelations` (`hostile[from][to]`), the single authority both
targeting and damage consult; the player-vs-world baseline is just its default.
Provoking a peaceful actor is **in-place** (`provoke_actor_in_place`): swap its
`Brain` + `ActionSet` and flip its disposition — **no entity churn**, no cluster
migration, sprite preserved. Dialogue/bark/interaction gate off a shared
`ActorInteraction` seam + disposition, never an actor type. Visual identity derives
from state (`is_sandbag → TrainingDummy`, `hostile → Enemy`, else `Npc`), so a
provoked NPC turns red automatically.

## What "done" means — convergence, not behavior

The acceptance test is **convergence: smaller, cleaner, better-organized code** — one
intent-resolver, one perception path, one relational damage model, one movement
pipeline, shared by the player and every actor; the parallel `Player*` clusters
retired or folded; **measurably less code than today**.

Behavioral scenarios are *evidence*, necessary but not sufficient (a scenario that
passes on two parallel paths has not met the bar):

- **Spam-equivalence** — a spam controller and a human produce the same physical
  output on the same body.
- **Drop-anywhere** — any body dropped at any position behaves reasonably; never
  wedges or leaves the world.
- **Spectator arena / mirror match** — two AI bodies (e.g. the player-robot and a
  boss), mutually hostile under one strong brain, fight without degenerate loops and
  route around a Neutral observer. Doubles as an out-of-bounds soak.
- **Possession + boss parity, in-game** — possess a body → its full moveset; drop the
  player-robot as a boss → its full moveset; one code path.
- **Frame-agnostic routing** — the strong brain fights and navigates correctly under
  C4 gravity and through portals.

## Where we are (shipped foundation)

The proof the convergence is reachable — already landed:

- **Shared movement spine + floating mover + blink + the directional block rule** —
  player and every actor share the core math.
- **Relational damage** — `FactionRelations` + a non-player-centric actor-vs-actor
  melee/projectile damage path; the player is a *gated* victim (a spectator-arena
  fighter spares the observer).
- **Headless perception value** — `WorldView` + `WorldMemory` (viewport, other
  actors, projectiles, terrain, line-of-fire / reachability over the *real* collision
  geometry), body-generic; built live and consumed by the brain (a line-of-fire gate
  stops the AI firing into walls).
- **The player kit as actor capabilities** — blink / fly / shield / dash resolve on
  an actor body as body-enforced capabilities; the `player_robot` archetype carries
  the full kit (droppable as a boss).
- **Movement physics as composable data** — `BodyMovementTuning` (per-archetype, with
  inheritance); the hardcoded `ENEMY_*` constants are gone.
- **The bridges to the player pipeline** — `ActorControlFrame::to_input_state()` and
  `BodyMovementTuning::spine_tuning()`, the seams an actor uses to run the player
  movement core.
- **The seam is proven** — a brain-driven, non-player entity already runs the *exact*
  player movement core in-game (`player_clone`): the player systems iterate every
  player-bodied entity from its `ActorControl`. A brain's `ActorControlFrame` already
  drives the player pipeline. The clone is "an enemy minus faction/combat."

## The path (elegance order)

The corrected direction (enemies rise to the player; delete-heavy):

1. **Movement tuning as data** ✅ — done (above).
2. **The bridges** ✅ — done (above).
3. **Route bodies through the player pipeline.** Give actor bodies the player
   movement clusters; in the per-actor update call `update_player_*_clusters(world,
   clusters, frame.to_input_state(), dt, tuning)` and **delete
   `integrate_standard_enemy_body`**. Enemies gain wall-cling / ledge-grab / dodge /
   variable-jump. Reconcile the two cases that don't fit the grounded mold:
   **surface-walkers** (the crawl path) and **aerial free-movers** (`gravity_scale`
   vs `PlayerFlightState`).
4. **Collapse the `Player*` / `Actor*` dual hierarchy** — the keystone. The player is
   today a ~20-module dependency sink; shared sim-state (`PlayerCombatState` →
   `ActorCombatState`, movement/ability states) moves onto the `Actor*` vocabulary in
   shared `body`/`actor` modules. **Sliced, not big-bang** — one component family per
   slice — gated only by *it compiles* + the differential headless trace. This is the
   prerequisite that unblocks crate extraction (see `engine/architecture.md`). Keep
   genuinely player-only things (HUD root, camera, device input, demo) on the player.
5. **De-player-center the remaining surface** — `ControlFrame` (global input) becomes
   entity-local `ActorIntent` (sim reads the body's intent, not a global; rendering /
   input-sources stay presentation consumers); projectile attribution moves from a
   player/enemy split to source/faction; `AggressionMode` names a faction, not "the
   player".
6. **Rename off type-names** — once player/enemy are `(controller, capabilities)`
   data, `enemy_archetypes.ron` / `EnemyArchetypeSpec` / `EnemyBrain` etc. are
   misnomers (these are *character* archetypes). A mechanical pass, done on its own.

> The `Player*`/`Actor*` collapse is feel-touching but **behavior is not sacred** —
> drive the real headless sim, diff a movement/combat trace before and after, accept
> changes that aren't egregious (often they're *better*), re-baseline canary tests.
> Get the shape right first; checkpoint each slice; keep moving.

## Pointers (verify before trusting — code moves)

- Input seam: `ambition_characters::brain` (`Brain`, the `smash/` strong brain),
  `actor/control.rs` (`ActorControlFrame`, `to_input_state`, `IntentOutcome`).
- Body resolver (actor side): `features/enemies/integration.rs`
  (`ActorMut::update`, the spine), `features/ecs/actors/update.rs`
  (`update_ecs_actors`), `combat/components` (`ActorAttackState`,
  `CombatCapabilities`, `ActorTuning`, `BodyMovementTuning`).
- The player pipeline to raise enemies onto: `ambition_engine_core/movement`
  (`update_player_*_clusters`, the `apply_*` limbs, `integrate_normal_spine`).
- Relational damage: `combat/targeting.rs` (`FactionRelations`), `combat/events.rs`
  (`HitTarget::Actor`), `combat/hitbox`, the projectile systems.
- Perception: `ambition_characters::perception` (`WorldView`, `WorldMemory`); built
  body-generic in the gameplay layer.
- The proven seam: `ambition_app/src/app/player_clone.rs` (+ the `clone_probe_tests`).
- Verify in the real sim: `SandboxSim::new_with_options(..).step(AgentAction)`;
  `ambition_app/tests/*` (dash_stability, blink_run_reachability, scripted_gameplay).

## Gotchas

- **Bevy 16-system-param + ~17-`.chain()` ceilings.** `update_ecs_actors` and the
  player-hit path are at them — bundle params into a tuple, register extra systems
  with explicit `.before/.after`.
- **`Block::solid` takes the MIN corner; `World::new` adds no boundary walls.** Test
  worlds author their own floor/walls.
- **Sim-time, not wall-time** (ADR 0010/0011): timers read `WorldTime::scaled_dt` /
  the accumulating `GameplayElapsed` so bullet-time and pause compose for free.
- **Query iteration order isn't stable** — sort order-sensitive passes by a stable id.
- **The player pipeline already consumes `ActorControlFrame`** (via `to_input_state`)
  — that is why an enemy carrying the movement clusters runs the player core with no
  new input plumbing.
