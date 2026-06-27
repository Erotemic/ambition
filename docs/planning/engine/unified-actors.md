# Unified actors

The heart of the engine. This consolidates the prior fighter-unification,
NPC/enemy-unification, non-player-centric, locomotion-split, and universal-brain
plans into one execution-grade statement of where actor control is going and how we
get there. It is a *plan*, not a changelog — the dated slice logs are gone; the
invariants, the execution order, the guardrails, and the gotchas are not.

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
nothing — the same body with a different controller.

## Invariants (Jon's words; non-negotiable)

Each rule operationalizes one of Jon's constraints. Keep the quotes — they are the
anchors that stop a future PR re-debating a settled decision.

- **I1 — One input seam for every controller.** Every controller emits an
  `ActorControlFrame`; `Human` / `Brain` / `Remote` / `RlPolicy` are mutually
  substitutable on any body.
  > "There needs to be a clean mapping between how the game-AI can make decisions and
  > how the human player, or some RL agent can make decisions. The input seam for all
  > of these should be the same."
- **I2 — Possession grants the body's full kit, nothing special-cased.**
  > "A human controller possessing a character should have the exact same capabilities
  > that a state machine brain does."
- **I3 — The body enforces; the controller only attempts.** Fire-rate, cooldown,
  stun, traction, which abilities exist — all on the body. If the only reason a body
  doesn't stream attacks is that the controller declines to, the design has failed.
  > "It is the body of the character that should limit things like fire-rate, not the
  > brain… The body imposes the physical constraints, and the brain attempts to give
  > inputs. It can receive feedback on when inputs are blocked by stun or cooldown,
  > but the brain is the controller not the enforcer."
- **I4 — Degenerate inputs are the world's problem, not the controller's.** The
  action space stays fully open (so an RL agent can probe it); cooldowns, the arena,
  and counterplay make lines uninteresting. Restraint is *policy* (hand-coded now,
  learnable later), never enforcement.
  > "If the brain could spam a continuous stream of gliders to auto-win fights, it
  > probably should… It's our job to constrain the world such that it's interesting."
- **I5 — Perception is a headless viewport, exactly like the human's.** A world-space
  region around the body, computed from gameplay state, **zero rendering dependency**.
  The camera may match a body's viewport; the camera consumes perception, never the
  reverse.
  > "Each character should be able to have a viewport into the entire world around
  > them, exactly like the human controlled character has (non-player centrism)."
  > "Do not couple perception to rendering. The game needs to run headless."
- **I6 — Bodies remember what they've seen.** Last-known positions with decaying
  confidence, so a controller can pursue a target that left its viewport.
  > "The brain should also have some memory of the larger space around them, even if
  > they can't see it, just like a human has."
- **I7 — Every body carries the full kit; the player-robot is droppable as a boss.**
  > "PCA should effectively have the expressive capabilities on par with the player
  > sprite… if the player robot isn't unified enough to be dropped in as a boss, then
  > that's a problem."
- **I8 — Drop a character anywhere and it behaves; the same placement runs in-game.**
  > "We should be able to drop a character in any location and have them behave
  > reasonably."
- **I9 — One strong, character-agnostic brain.**
  > "We need one really strong AI brain that can control generic characters and always
  > provide a challenge."
- **I10 — Frame-agnostic throughout (relativity).** Perception, motor, and abilities
  live in the acceleration frame — correct under rotated (C4) gravity and through
  portals. No code assumes `-y` is up.

## Architecture

### The two ports

Every body exposes exactly two ports; every controller plugs into the same two:

- **intent-in** — the body resolves an `ActorControlFrame` into effects, enforcing
  its own physics and returning per-intent *accepted | blocked* feedback
  (`IntentOutcome`). Movement is just another intent.
- **world-out** — the body produces a controller-neutral, **headless** `WorldView`
  (what it can perceive) plus a `WorldMemory` (what it has seen).

The brain pipeline is one path regardless of backend:
**`Brain` → `ActionSet` / `ActorControl` → `ActorActionMessage`** (action consumers —
melee, projectile, boss specials — read the message channel). Player movement rides
`ActorControl`, *not* a player-only seam. Never leak ECS/world access into a brain's
tick — that keeps brains pure, replayable, and RL-droppable. Prefer **enum dispatch
over `dyn Brain`** in hot paths (easier to profile, batch, and serialize).

**Brain backends** — a design vocabulary, not a checklist of done-ness:

| Backend | Intended use |
|---|---|
| `Player` | Human / controller input. |
| `Remote` | Networked or replayed control frames. |
| `RlPolicy` | Batched inference / training. **Add only when a concrete consumer lands — no speculative FFI.** |
| `StateMachine` | NPC / enemy-style AI (the Smash strong brain lives here). |
| `BossPattern` | Generic boss-pattern driver with named boss data above it. |
| `Scripted` | Cutscene / authored input tracks. |

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
  free-mover / choreography modality: boss patterns that snap to a path, AI flyers
  steering 2D directly. The floating integrator reads this; grounded reads
  `locomotion`. Each consumer picks the field for its mode.

> **Closed decision:** this dual meaning is **essential complexity, not debt**. Do
> NOT "fix" it by decomposing velocity back into intent per tick. The consumer
> pattern for a grounded body is exactly:
> `integrate_normal_spine(.., InputState { axis_x: frame.locomotion.x, .. }, ..)` with
> `MovementTuning { max_run_speed, .. }`. Correct behavior is emergent from this
> structure, not preserved by a per-tick `max_run_speed = |desired|` decomposition.

### One spine, one floating mover

- Grounded bodies run the **shared spine** `integrate_normal_spine` (+
  `NormalSpineCtx::bare` for bodies without ability limbs): gravity-relative gravity,
  run, fall-cap, fast-fall/glide gates — pay-for-use.
- Aerial bodies run the shared **`step_floating_body`** (`accel: None` = snap to a
  pattern).
- **Platform riding is emergent** — blocks carry velocity; there is no rider list.
- Movement physics is **per-body data** (`BodyMovementTuning`, composed hierarchically
  per archetype, with `inherits`); the hardcoded `ENEMY_*` constants are gone.

> **Closed decisions — do not re-attempt:** (1) the three grounding sweeps
> (gravity-resting, surface-glued/crawl, the shared rule) are genuinely different
> physics — do **not** collapse them into one wide generic surface. (2) Do **not**
> component-ize the per-archetype capability flags into separate marker components;
> content data opting in already satisfies composability.

### Relational hostility, in-place provoke

Hostility is `FactionRelations` (`hostile[from][to]`), the single authority both
targeting and damage consult; the player-vs-world baseline is just its default.
Provoking a peaceful actor is **in-place** (`provoke_actor_in_place`): swap its
`Brain` + `ActionSet` and flip its disposition — **no entity churn**, no cluster
migration, sprite + `ActorRenderSize` preserved (the balloon-bug class is gone by
construction). Dialogue / bark / interaction gate off a shared `ActorInteraction`
seam (`Interactable` + `talk_radius`) + `ActorDisposition::Peaceful`, never an actor
type. Visual identity derives from state (`is_sandbag → TrainingDummy`, `hostile →
Enemy`, else `Npc`), so a provoked NPC turns red automatically.

## What "done" means — convergence, not behavior

**Convergence is the acceptance test — behavior alone is necessary but not
sufficient.** A scenario that passes on *two parallel paths* has not met the bar (you
could fake the spectator arena with two copies of the enemy path). The real bar is
**smaller, cleaner, better-organized code**: one intent-resolver, one perception path,
one relational damage model, one movement pipeline, shared by the player and every
actor; the parallel `Player*` clusters retired or folded; **measurably less code than
today**.

> A slice that adds an actor capability without moving the player onto the shared
> path has spent effort without converging. Track it, but it is not the goal.

Behavioral scenarios are *evidence*:

- **Spam-equivalence** (I3) — a spam controller and a human produce the same physical
  output on the same body.
- **Drop-anywhere** (I8) — any body dropped at any position behaves reasonably; never
  wedges or leaves the world.
- **Spectator arena / mirror match** (I9) — the player-robot and a boss, mutually
  hostile under one strong brain with a Neutral observer, fight without degenerate
  loops and route around the observer. Doubles as an out-of-bounds soak.
- **Possession + boss parity, in-game** (I2, I7) — possess a body → its full moveset;
  drop the player-robot as a boss → its full moveset; one code path.
- **Frame-agnostic routing** (I10) — the strong brain fights and navigates correctly
  under C4 gravity and through portals.

## Convergence audit (the debt baseline)

Where things actually stand, so progress is measurable and a regression is
recognizable:

- **Foundation — unified.** The movement spine, blink, the directional block rule,
  `AccelerationFrame` / `BodyKinematics`, frame-agnostic perception/motor. The proof
  the convergence is reachable.
- **Orchestration — grounded movement now CONVERGED; abilities still duplicated.**
  The grounded actor runs the player movement pipeline directly
  (`ActorMut::integrate_grounded_body` → `update_body_with_tuning_clusters`,
  borrowing `kin` + the new `ActorBody` clusters); `integrate_standard_enemy_body`
  is deleted. Still duplicated: blink/fly/shield/dash exist **twice** — the actor's
  copies live on `ActorAttackState` / `CombatCapabilities`, NOT yet the pipeline's
  ability limbs (so the actor ability mask stays locomotion-only). Folding those
  onto the limbs + retiring `ActorSurfaceState`'s redundant ground/jump fields is
  the step-4 collapse. Aerial free-movers + surface-walkers still run their own
  steps by design.
- **Targeting — relational.** `FactionRelations` + `select_actor_targets`; an Enemy
  targets an Npc with no player present. *Gap:* the player is still an unconditional
  candidate; `AggressionMode::HostileToPlayer` names the player.
- **Damage routing — relational** (`HitTarget::Actor`, projectiles by firer faction);
  the player is a *gated* victim.
- **Player-robot as an actor — exists** (`player_robot` archetype, full kit). Building
  it is what *forced* the player kit to become `CombatCapabilities`.
- **Naming — still player-centric.** `enemy_archetypes.ron` / `EnemyArchetypeSpec` /
  `EnemyBrain` are misnomers (these are *character* archetypes).

## Where we are (shipped foundation)

Shared spine + floating mover + blink + directional block rule; relational damage
(`FactionRelations`, actor-vs-actor melee/projectile, player a gated victim); headless
`WorldView` + `WorldMemory` (built body-generic by `build_world_view(body, …)` — *one*
function for a Player-faction view and an Enemy-faction view, hostility resolved from
`FactionRelations`, not the viewer's type), with a line-of-fire gate that stops the AI
firing into walls; the player kit as actor capabilities (blink/fly/shield/dash) + the
`player_robot` archetype; movement physics as composable data; the two bridges
(`ActorControlFrame::to_input_state`, `BodyMovementTuning::spine_tuning`).

**The seam is proven.** `ambition_app/src/app/player_clone.rs` (+ `clone_probe_tests`)
spawns a non-player entity carrying the player movement clusters + a Brain →
`ActorControl`; the iterating `player_control_system` / `player_simulation_system`
already run it through the *exact* player movement core (it is a `PlayerEntity`, not
`PrimaryPlayer`, not `PlayerSlot`). **A Brain's `ActorControlFrame` already drives the
player pipeline.** The clone is "an enemy minus faction/combat" — so the rest of the
refactor is *make actors look like the clone*, not invent new seams. (One caveat the
clone exposes: the *live* player still consumes raw input today; step 4 must make the
live player consume `ActorControl` too — that is what makes it droppable as a boss.)

## The path (elegance order, with acceptance)

Enemies rise to the player; delete-heavy. Each step is gated on *it compiles* (incl.
`ambition_app`) + invariants hold; behavior may change.

1. **Movement tuning as data** ✅ — `BodyMovementTuning` per archetype + inheritance;
   `ENEMY_*` constants deleted. *Done when:* an archetype with no `movement:` row
   resolves to the baseline (behavior-preserving); an authored override changes its
   physics, proven headless.
2. **The bridges** ✅ — `to_input_state` + `spine_tuning`. *Done when:* both are
   tested and the spine runs off `spine_tuning` (byte-identical).
3. **Route bodies through the player pipeline.** 🟡 *grounded done.* First the
   movement core was made body-generic — `update_body_*_with_clusters` flag
   hazard/drown/OOB as `FrameEvents` WITHOUT performing the player respawn; the
   `update_player_*` entries are thin wrappers = body fn + the respawn policy, so
   the core no longer teleports any body to the player spawn (the step-2
   prerequisite, since an actor must own its hazard reaction). Then the grounded
   actor was routed onto it: a new `ActorBody` component carries the 18 ancillary
   player movement clusters (everything but `BodyKinematics`, which stays the
   shared `kin` — no duplication); `ActorMut::integrate_grounded_body` borrows
   `kin` + `ActorBody` as one `PlayerClustersMut` and drives
   `update_body_with_tuning_clusters`. **`integrate_standard_enemy_body` is
   deleted** — its aerial half is `integrate_aerial_body` (still
   `step_floating_body`). A grounded enemy now runs / buffers-and-coyote-jumps /
   collides through the EXACT player core. *Remaining:* (a) the ability mask is
   locomotion-only — enabling wall-cling / ledge-grab / dodge for actors, and
   folding the actor's own dash / blink / fly / shield onto the pipeline limbs
   (currently still on the `ActorAttackState` path), is the step-4 cluster
   collapse; (b) the **aerial free-mover** (`gravity_scale` vs `PlayerFlightState`)
   and **surface-walker** modalities are still reconciled separately, as planned.
   *Gotcha (held):* `ActorMotionPath` patrol + `gravity_scale`-from-catalog
   survive the merge.
4. **Collapse the `Player*` / `Actor*` dual hierarchy** — *the keystone* (see
   [`architecture.md`](architecture.md) for the slice plan + component buckets). Move
   shared sim-state onto the `Actor*` vocabulary; the ~20-module player dependency
   sink dissolves and crate extraction unblocks. **Sliced, not big-bang** — one
   component family per slice, ordered low→high feel-risk (economy/interaction first,
   combat state next, movement/ability state **last**), each gated on compile + a
   byte-stable differential trace. *Done when:* the duplication in the audit is
   provably gone (one resolver, one perception path, one damage model — measurably
   less code); possession works in-game; the player-robot drops as a boss.
5. **De-player-center the remaining surface** — `ControlFrame` (global input) →
   entity-local `ActorIntent` (~46 systems read the global today; the sim reads the
   body's intent, rendering / input-sources stay presentation consumers); projectile
   attribution → source/faction (track enemy-projectile owners by entity);
   `AggressionMode` names a faction, not "the player".
6. **Rename off type-names** — `enemy_archetypes.ron` / `EnemyArchetypeSpec` /
   `EnemyBrain` → *character* archetypes. A mechanical pass on its own; update the
   `architecture_boundaries` guard test if it asserts names.

### Deferred (on purpose — not blocked, just not now)

- **`special` / signature moves** — the `special_pressed` seam resolves, but no body
  authors a signature yet (authoring one is "new ability" content, a non-goal of the
  unification). Leave the seam inert.
- **Folding the player `ProjectileSpawner`** (cooldown + mana meter + charge state
  machine) onto `try_fire_ranged` — feel-sensitive (changes player feel); part of the
  step-4 collapse, done behind the differential trace, not blind.

## Guardrails — do not make the keystone harder

Every later step must move toward convergence, never away:

1. **Build perception body-generic from day one.** `WorldView` / `WorldMemory` are
   functions of a **body** (any faction). Do NOT hang perception off the enemy-only
   snapshot path or key it on `EnemyBrain`. "Perception for the player" means a brain
   driving the player-robot body gets the same `WorldView`.
2. **The strong brain takes a body + its `WorldView`**, never an actor-only or
   enemy-only type. If it can't drive the player-robot body, it's the wrong shape.
3. **Add no new `"player"`-string couplings or `Player*`-only clusters.** New
   capability/state goes on the shared `CombatCapabilities` / `ActorAttackState` /
   `ActorStatus` vocabulary; route damage through `FactionRelations`.
4. **Relational damage is additive — don't "finish" it by ripping things out.** The
   player's OWN attacks stay universal (so striking a peaceful NPC provokes it);
   hazards / pogo / charge-crash / breakables are deliberately NOT faction-gated.
5. **The keystone is a checkpointed refactor, not a blind rewrite.** Build the parity
   net first (the trace tooling — see [`headless-verification.md`](headless-verification.md));
   capture a trace before a feel-touching slice, diff after. Replay/feel may change —
   only the compile + the feel diff gate it. Commit = checkpoint, keep moving. Jon
   verifies feel in-game; ship a feel-sensitive change blind in its own marked commit
   and ask.

## Pointers (verify before trusting — code moves)

- Input seam: `ambition_characters::brain` (`Brain`, the `smash/` strong brain),
  `actor/control.rs` (`ActorControlFrame`, `to_input_state`, `IntentOutcome`).
- Body resolver (actor side): `features/enemies/integration.rs` (`ActorMut::update`,
  the spine), `features/ecs/actors/update.rs` (`update_ecs_actors`), `combat/components`
  (`ActorAttackState`, `CombatCapabilities`, `ActorTuning`, `BodyMovementTuning`).
- Player pipeline to raise enemies onto: `ambition_engine_core/movement`
  (`update_player_*_clusters`, the `apply_*` limbs, `integrate_normal_spine`,
  `PlayerClustersMut`).
- Relational damage: `combat/targeting.rs` (`FactionRelations`), `combat/events.rs`
  (`HitTarget::Actor`), `combat/hitbox`, the projectile systems.
- Perception: `ambition_characters::perception` (`WorldView`, `WorldMemory`,
  `build_world_view`).
- The proven seam: `ambition_app/src/app/player_clone.rs`, `player::clone_probe_tests`.
- Verify in the real sim: `SandboxSim::new_with_options(..).step(AgentAction)`;
  `ambition_app/tests/*`. Detailed specs: `docs/systems/brain-driver.md`,
  `docs/recipes/extending-brains-and-action-sets.md`, `docs/adr/0016-actor-unification.md`.

## Gotchas (hard-won)

- **Bevy 16-system-param ceiling.** `update_ecs_actors` and the player-hit path are at
  it — bundle params into a tuple `(a, b): (Res<A>, Res<B>)` rather than adding a slot.
  The `.chain()` length ceiling (~17) is real too — register the extra system with an
  explicit `.before/.after`.
- **`Block::solid(name, min, size)` takes the MIN corner, not the center; `World::new`
  adds NO boundary walls.** Test worlds author their own floor/walls.
- **Sim-time, not wall-time** (ADR 0010/0011): timers read `WorldTime::scaled_dt` / the
  accumulating `GameplayElapsed` so bullet-time and pause compose for free.
- **Query iteration order is NOT stable** — sort order-sensitive passes by a stable id
  (`config.id` / `owner_id`), not `Entity`.
- **Bevy 0.18 Message API** — buffered events are `Message` (`MessageReader` /
  `MessageWriter` / `add_message`); the old `Event` is observer-only.
- **The cluster merge is atomic.** Routing through the player pipeline (step 3) and the
  step-4 collapse produce no intermediate green tree until the family is fully moved —
  budget for a long compile-error chase; don't commit a half-merged tree.
- **Feel-unverified flags Jon must check in-game after a change:** the dash tuning
  (`ActorAttackState::DASH_SPEED_MULT = 1.7`, `DASH_TIME_S = 0.18`, refire `0.7s`); the
  `player_robot` archetype movement tuning; the shield-fold; a same-room reset keeps a
  provoked NPC hostile (the in-place peaceful-revert is a noted follow-up, not a bug);
  idle/hit/hostile barks still fire (parrot cove).
