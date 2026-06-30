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
targeting and damage consult; the player-vs-world baseline is just its default. The
player is just `ActorFaction::Player` — **not** an unconditional targeting candidate
(B1 end-state): an actor targets the nearest member of any faction it opposes, full
stop. The two old modes (`HostileToPlayer` / `HostileToFaction`) collapse into that
one relational policy; the "spared observer" falls out of relations simply not making
the observer a foe. Provoking a peaceful actor is **in-place**
(`provoke_actor_in_place`): swap its `Brain` + `ActionSet`, flip its disposition, and
record a **per-actor grudge** against the attacker (generalize the existing `strikes`
accumulator off the hardcoded player) — **no entity churn**, no cluster migration, no
faction-identity mutation, sprite + `ActorRenderSize` preserved (the balloon-bug class
is gone by construction). Grudges are friendly-fire-gated: FF-off means allies never
land a hit to provoke from, FF-on lets an actor hold a grudge against whoever hit it. Dialogue / bark / interaction gate off a shared `ActorInteraction`
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
- **Orchestration — movement CONVERGED.** Every actor (grounded AND aerial) runs
  the ONE shared player movement pipeline (`ActorMut::integrate_body` →
  `update_body_with_tuning_clusters`); both bespoke integrators are deleted; run /
  jump / dash / blink / fly / shield are folded onto the pipeline's ability limbs
  (mask from `CombatCapabilities`); `update_ecs_actors` resolves no movement verb;
  aerial bodies steer the flight limb via the `velocity_target`→intent bridge.
  Surface-walkers keep a separate glued crawl by design.
- **Sim-state — CONVERGED (Phase A, step 4).** Every body-fact now has ONE authority
  on the shared body: `ActorStatus`'s parallel `alive` / `damage_invuln_timer` /
  `hit_flash` retired onto `BodyHealth` / `BodyCombat` (the SAME fields the player
  carries), and `ActorSurfaceState`'s `on_ground` / `air_jumps` retired onto
  `BodyGroundState` / `BodyJumpState`. `ActorStatus` is down to `{respawn_timer,
  ai_mode}` (genuinely actor-only). Bosses keep a separate `BossStatus` (its own
  alive/health/hit_flash) — a parallel island, a later slice.
- **Targeting — relational, decision made (step 5/B1).** `FactionRelations` +
  `select_actor_targets`; an Enemy targets an Npc with no player present. *Player-
  centrism to remove:* the player is an unconditional candidate and
  `AggressionMode::HostileToPlayer` names the player. **Decision:** collapse to ONE
  relational policy (target nearest member of any faction I oppose; player is just
  `ActorFaction::Player`); provoke is a **per-actor grudge** (generalize the existing
  `strikes` accumulator from the hardcoded player to the attacker). Grudges are
  naturally gated by friendly-fire: with FF off, allies/observers never land a hit, so
  no spurious grudge forms; with FF on, an actor can hold a grudge against whoever hit
  it.
- **Damage routing — relational** (`HitTarget::Actor`, projectiles by firer faction);
  the player is a *gated* victim. **Projectile world-hit decision (step 5/B2):**
  `WorldHitPolicy` moves onto the projectile **spec** (authored per ability), not the
  binary `Player`/`Enemy` faction — so a Hadouken behaves identically whoever fires it
  (the player or the player-robot boss). Retire `ProjectileFaction` (owner entity →
  faction for damage, already there).
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
3. **Route bodies through the player pipeline.** ✅ *movement converged.* Every
   actor (grounded + aerial) runs the ONE pipeline; both bespoke integrators are
   deleted; dash/blink/fly/shield are folded onto the ability limbs (mask from
   `CombatCapabilities`); the aerial `velocity_target`→intent bridge lets flyers
   share the flight limb. Surface-walkers stay a separate glued crawl by design.
   *Still additive (not blocking):* wall-cling / ledge-grab / dodge for actors —
   each needs a `CombatCapabilities` cap (contact-triggered → all-or-nothing without
   a gate); actors already *animate* these poses (anim picker converged), so flipping
   the caps makes them mechanically real.
4. **Collapse the `Player*` / `Actor*` dual hierarchy** — *the keystone*. 🟢 **State
   collapse (Phase A) DONE** — the body-vocab types (`BodyKinematics`, the 18 movement
   clusters, `BodyHealth`, `BodyCombat`, `BodyWallet`) were re-homed off `crate::player`
   in prior work, and Phase A retired the last duplicated actor-only STATE fields:
   `ActorStatus.{alive,damage_invuln_timer,hit_flash}` → `BodyHealth`/`BodyCombat`,
   `ActorSurfaceState.{on_ground,air_jumps}` → `BodyGroundState`/`BodyJumpState`. One
   authority per body-fact. *Remaining for full step-4 done:* the broad `crate::player`
   importer-sink metric + possession working in-game + the player-robot dropping as a
   boss (these are largely VERIFICATION now — possession systems + the `player_robot`
   archetype already exist; confirm end-to-end + measure the LOC/importer delta). See
   [`architecture.md`](architecture.md) for the component buckets.
5. **De-player-center the remaining surface** — decisions settled with Jon (2026-06-30):
   - **`ControlFrame` → entity-local `ActorIntent` (B3).** *Boundary principle (agreed):*
     sim/body systems read the body's entity-local intent; only input-source adapters
     and presentation read the global `ControlFrame`. Per-reader audit + repoint of the
     ~22 `Res<ControlFrame>` sim readers (MIGRATE: shrine, shockwave, affordances/intent,
     possession; KEEP: dialog, menus, mobile-input, portal input-adapter, rl_sim,
     item_visuals), each gated on the byte-stable player replay trace. **Put `ActorIntent`
     on the player too** (Jon's call) — the player and an NPC expose the SAME intent
     component, so migrated sim reads consume one body-generic seam, not `PlayerInputFrame`.
   - **Projectile attribution → spec + owner (B2).** `WorldHitPolicy` moves onto the
     projectile spec (per-ability, firer-agnostic); retire the binary `ProjectileFaction`
     (owner entity → faction for damage).
   - **`AggressionMode` → fully relational (B1).** One policy via `FactionRelations`;
     provoke = per-actor grudge (FF-gated). See "Relational hostility" above.
     *Concrete design (scoped 2026-06-30):* add `grudge: Option<ActorFaction>` to
     `ActorAggression`. Collapse `AggressionMode` → `{Passive, RetaliatesWhenHit, Hostile}`
     and `AggressionTarget` → `{None, Foe}`. The hostility test becomes
     `is_hostile(from, to) || grudge == Some(to)`, consulted at the TWO sites the
     provoke faction-flip currently feeds: `select_actor_targets` (so the actor
     *chases* the foe) and `apply_player_hit_events` (the player-victim gate, so its
     hits *land*). Provoke (`apply_actor_stimuli`) sets `grudge = attacker faction` +
     disposition Hostile instead of flipping the NPC's faction to Enemy (no identity
     mutation). FF-gating is emergent: with FF off, `can_damage` blocks ally-on-ally
     hits → no `DamagedBy` stimulus → grudges only ever form against real attackers
     (the player, via its universal attacks). A born-hostile Enemy needs no grudge
     (faction relations already make it hunt the player). *Open micro-decision:*
     grudge keyed by **faction** (simplest, correct single-player; over-generalizes in
     MP — provoking one NPC angers it at all Player-faction bodies) vs by **entity**
     (precise, but the player-victim gate is faction-based so it needs a small bridge).
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

These next ones are **features we are NOT building yet** — but the architecture must be
shaped *now* so that when we do, they land on the shared body vocabulary instead of
forking a fourth/fifth player/NPC/enemy/boss path. Each entry names the body-generic
seam the future feature MUST use:

- **NPC agency — body-generic interaction (the consumer side).** The interaction
  *intent* is already non-player-centric: `ActorControlFrame.interact_pressed` exists
  ("brain wants to interact with whatever is nearby"), so any brain can emit it. What is
  still player-only is the **consumer**: `PlayerInteractionState` is a human double-tap /
  button *input buffer* (genuinely controller-side, keep it there), and the affordance /
  interaction systems (doors, NPCs, pickups, ground items) run only for the player. *When
  we add NPC world-interaction:* lift those systems to act on **any body whose intent has
  `interact_pressed`**, resolving against the same affordance proximity model — NOT a new
  `Npc*` interaction path. The human's double-tap buffer simply *produces* `interact_pressed`
  like any other controller; the resolver downstream is body-generic.
- **Barks / ambient dialog (no time-stop).** `VfxMessage::SpeechBubble` already renders a
  line over any body (used by the actor hit-bark path). *When we add NPC↔world / NPC↔NPC
  conversation:* model it as a **body-level bark channel driven by a brain**, not a
  player-gated dialog. Time pauses **only** for an explicit cutscene; ambient barks never
  pause. The blocking Yarn runner stays the cutscene path; ambient lines are a separate
  non-blocking emit. (Open design fork — queued line on the body vs. a lightweight
  two-brain "conversation" pairing vs. a non-blocking Yarn mode — decide before building.)
- **Economy as a body concern.** `BodyWallet` is now body vocabulary (commit landed). *When
  we add drops / trading NPCs / multiplayer currency:* an NPC that drops currency carries a
  `BodyWallet`; currency pickups credit **a body** (proximity / owner-resolved in MP), not a
  global "the player". Do not reintroduce a player-only economy resource.
- **Multiplayer — per-body, never global-singular.** Per-player state (`BodyWallet`,
  camera, device input, `PlayerSlot`) is already per-entity; the remaining single-player
  assumptions are *global resources / `.single()` player queries* (e.g. currency pickup
  attribution, the global `ControlFrame` — see step 5's `ActorIntent`). *When we add a
  second player:* it is just another `PlayerEntity` body with its own controller; nothing
  in the sim may assume one. This is the same shape as possession (a human controller on
  any body), so building one builds the other.

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
6. **Body-generic *consumers*, not just body-generic *state*.** A unified component is
   only half the win — the SYSTEMS that read it must run for any body too. The recurring
   trap: the intent/state is already shared (`ActorControlFrame.interact_pressed`,
   `BodyWallet`, `SpeechBubble`) but its consumer system is gated `With<PlayerEntity>` or
   keyed on the primary player. Before adding a feature on a shared component, check its
   consumer: if it's player-gated, the feature would fork an `Npc*` twin. Lift the
   consumer to query the body vocabulary (faction-filter where hostility matters) instead
   of adding a parallel path. "Could an NPC brain trigger this with no new system?" is the
   test — if no, the consumer is the bifurcation, fix it first.

## Pointers (verify before trusting — code moves)

- Input seam: `ambition_characters::brain` (`Brain`, the `smash/` strong brain),
  `actor/control.rs` (`ActorControlFrame`, `to_input_state`, `IntentOutcome`).
- Body resolver (actor side): `features/enemies/integration.rs` (`ActorMut::update`,
  the spine), `features/ecs/actors/update.rs` (`update_ecs_actors`), `combat/components`
  (`ActorAttackState`, `CombatCapabilities`, `ActorTuning`, `BodyMovementTuning`).
- Body pipeline both run on: `ambition_engine_core/movement`
  (`update_body_with_tuning_clusters` + the `update_player_*_clusters` wrappers = body fn +
  respawn policy, the `apply_*` limbs, `integrate_normal_spine`, `BodyClustersMut` — the
  view both the player query and `ActorMut::clusters_mut` build; module `body_clusters`).
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
- **Feel-unverified flags Jon must check in-game after a change:** the `player_robot`
  archetype movement tuning; the shield-fold; a same-room reset keeps a provoked NPC
  hostile (the in-place peaceful-revert is a noted follow-up, not a bug); idle/hit/
  hostile barks still fire (parrot cove). *(Verified by Jon 2026-06-30:* the Phase-A
  hit-flash collapse — enemy/NPC damage-blink + respawn blink read fine.*)*  After
  Phase A's A3, grounded-actor air-jump refresh + the flying-never-grounded guard apply
  same-tick on the shared cluster (no behavior regression observed in headless).
