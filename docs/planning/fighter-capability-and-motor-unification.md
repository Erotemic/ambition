# Unified character control & perception

A roadmap for the architecture in which **any controller drives any body through
one input seam, the body enforces all physics, perception is one headless
world-view, and a single strong brain can fight as any character.**

Author model: Opus 4.8 (1M).

---

## The model: a body has two ports

Every **body** exposes exactly two ports, and every **controller** — a human's
input, a hand-authored brain, or a learned policy — plugs into the same two:

- **intent-in** — the body resolves an `ActorControlFrame` into effects,
  enforcing its own physics (cooldown / stun / resource / traction / which
  abilities exist) and returning per-intent *accepted | blocked* feedback.
- **world-out** — the body produces a controller-neutral, **headless**
  `WorldView` (what it can perceive) plus a `WorldMemory` (what it has seen).

The controller only *attempts* inputs and *observes*; the body owns the rules.
A **real-ECS harness** runs bodies + controllers + a real room headless to prove
it. Get the two ports right with the player-robot and the Perfect Cell-ular
Automaton (PCA) as the first two bodies, and a third / fourth character, plus
possession and AI bosses, cost almost nothing.

## Invariants (Jon's constraints are authoritative; each rule operationalizes one)

**I1 — One input seam for every controller.**
> "There needs to be a clean mapping between how the game-AI can make decisions
> and how the human player, or some RL agent can make decisions. The input seam
> for all of these should be the same." — Jon

Every controller emits an `ActorControlFrame`; `Human` / `Brain` / `Remote` /
`RlPolicy` are mutually substitutable on any body. This seam already exists.

**I2 — Possession grants the body's full kit, nothing special-cased.**
> "A human controller possessing a character should have the exact same
> capabilities that a state machine brain does." — Jon

Abilities are resolved from the frame against the *body's* capabilities, never
gated on "is this the player."

**I3 — The body enforces; the controller only attempts.**
> "It is the body of the character that should limit things like fire-rate, not
> the brain… The body imposes the physical constraints, and the brain attempts to
> give inputs. It can receive feedback on when inputs are blocked by stun or
> cooldown, but the brain is the controller not the enforcer." — Jon

If the only reason a body doesn't stream attacks is that the controller declines
to spam, the system has failed — a human could spam it. The body is the floor.

**I4 — Degenerate inputs are the world's problem, not the brain's.**
> "If the brain could spam a continuous stream of gliders to auto-win fights, it
> probably should, or at least have the capacity for an RL agent to find that
> degenerate solution. It's our job to constrain the world such that it's
> interesting." — Jon

The action space stays fully open (so RL can probe it); body cooldowns, the
arena, and counterplay — not a hobbled controller — make lines uninteresting.

> "The only constraint on the brain might be to back off and regroup instead of
> being aggressive all the time (and our initial hand crafted policies can hard
> code that), but that's probably something that should also be able to be
> emergent by an optionally learned RL agent because it will be optimal for the
> constraints set by the world." — Jon

So restraint is *policy* (hand-coded now, learnable later), never enforcement.

**I5 — Perception is a headless viewport, exactly like the human's.**
> "Each character should be able to have a viewport into the entire world around
> them, exactly like the human controlled character has (non-player centrism).
> Each brain should have the capacity to observe the entire visible world around
> their viewport." — Jon

The viewport is a world-space region around the body (the AI analogue of the
player's screen), computed from gameplay state in the acceleration frame.

> "Do not couple perception to rendering. The game needs to run headless." — Jon

`WorldView` has **zero rendering dependency**. The render camera may be framed to
match a body's viewport for display, but the camera consumes perception, never
the reverse.

**I6 — Bodies remember what they've seen.**
> "The brain should also have some memory of the larger space around them, even
> if they can't see it, just like a human has. If the player goes off screen and
> the goal is to attack the player, the brain might try to move towards the last
> known position of the player to look for them so they can continue the onslaught
> (assuming they are very aggressive)." — Jon

`WorldMemory` carries last-known actor positions (decaying confidence) and seen
terrain, so a controller can pursue a target that left its viewport.

**I7 — Every body carries the full kit; the player-robot is droppable as a boss.**
> "PCA should effectively have the expressive capabilities on par with the player
> sprite… ledge grab, blink, fly, special projectiles, attack tilts, everything…
> if the player robot isn't unified enough to be dropped in as a boss, then that's
> a problem." — Jon

The player kit is a per-body capability set, resolved from the frame by any
controller.

**I8 — Drop a character anywhere and it behaves; the same placement runs in-game.**
> "We should be able to drop a character in any location and have them behave
> reasonably." — Jon

A test placement and an in-game placement are the same thing — there is no
special "scenario" format.

**I9 — One strong, character-agnostic brain.**
> "We need one really strong AI brain that can control generic characters and
> always provide a challenge." — Jon

Simpler brains may consume a subset of the seams; one brain must handle the full
kit, perception, and routing on any body.

**I10 — Frame-agnostic throughout.** Perception, motor, and abilities live in the
acceleration frame, so everything is correct under rotated (C4) gravity and
through portals. No code assumes `-y` is "up." (The relativity principle.)

## The end state, and what "done" actually means

The target is not "actors gained the player's moves." It is **one implementation**
that the player and every actor share:

- One intent-resolver. The player body writes an `ActorControlFrame` and runs the
  **same** resolution path as every actor — blink / fly / shield / dash / fire /
  tilts / special resolve once, against the body's capabilities. The parallel
  player-only clusters (`PlayerShieldState`, `PlayerDashState`, `PlayerFlightState`,
  the player `ProjectileSpawner`) are **retired or folded** into the shared
  capability state, not mirrored.
- One perception path (`WorldView` + `WorldMemory`), built for the player exactly
  as for any actor.
- One damage model. Who-hits-whom is **relational** (the `FactionRelations`
  matrix), not the player-vs-enemy bipartite split that hitboxes and projectiles
  hard-code today.
- The player kit *is* actor `CombatCapabilities`. The player-robot is therefore
  expressible as an actor body and **droppable as a boss** (I7) with no new
  plumbing, and **possession works in-game** because possessing is just swapping
  the controller on a body that already runs the one path.

> **Possession does not work in-game yet — by choice.** It is the *payoff* of this
> unification, deliberately not pushed until the player is genuinely one of the
> bodies. Possession "working" on the duplicated stack would be a bridge hack, not
> the real thing.

**Convergence is the acceptance test — behavior alone is necessary but not
sufficient.** The spectator arena (two AI bodies fight, observer ignored) proves
the *behaviors* compose non-player-centrically, but you could fake it with two
copies of the enemy path. The real bar is **smaller, cleaner, better-organized
code**: fewer parallel implementations, one resolver, one perception path, one
damage model — the duplication catalogued in the audit below provably *gone*. A
slice that adds an actor capability without moving the player onto the shared path
has spent effort without converging; track it, but it is not the goal.

> Calibration (Jon, 2026-06-26): "The spectator arena is necessary for acceptance,
> but not sufficient. Full unification convergence is the acceptance test with
> cleaner — smaller — better organized code." And: "as long as we keep notes of
> these and rapidly move towards convergence it's fine."

### Convergence audit (2026-06-26)

Honest snapshot of how unified things are right now, so the remaining work is
debt-paydown toward the end state above, not open-ended feature-add:

- **Foundation — unified.** Player and every actor share the movement spine
  (`integrate_normal_spine`), blink (`blink_target`), the block rule
  (`shield_blocks_hit`), `AccelerationFrame` / `BodyKinematics`. Frame-agnostic
  (S2). This layer is the proof the convergence is reachable.
- **Orchestration — duplicated (the debt).** The player runs its own pipeline
  (`ambition_engine_core/movement`) with `PlayerShieldState` / `PlayerDashState` /
  `PlayerFlightState` / `ProjectileSpawner`; actors run
  `integrate_standard_enemy_body` with the *parallel* `CombatCapabilities` +
  `ActorAttackState`. Blink/fly/shield/dash now exist **twice**, sharing core math
  but not implementation. S3 built the actor half to parity; **it did not yet
  retire the player half** — that is the convergence work.
- **Targeting — already relational (non-player-centric).** `combat/targeting.rs`
  has a `FactionRelations` matrix + `select_actor_targets`; an Enemy can target an
  Npc with no player present (tested). *Gap:* the player is still an
  **unconditional** candidate in the pool, and `AggressionMode::HostileToPlayer`
  names the player — so "ignore the player" / "hostile to faction X" is not yet
  expressible.
- **Damage routing — bipartite (the biggest player-centrism left).** Melee
  hitboxes (`combat/hitbox/mod.rs`) and projectiles (`ProjectileFaction =
  Player | Enemy`) only route player-side ↔ enemy-side. A PCA can *target* a
  robot but **cannot damage it** — there is no Enemy-damages-Boss path. This must
  become relational (drive off `FactionRelations`) for the arena and for honest
  non-player-centric combat.
- **Player-robot as an actor — does not exist.** The protagonist is only the
  bespoke player entity; there is no actor archetype for it (blocks I7 + the arena).
  Building it is what *forces* the player kit to become `CombatCapabilities`.
- **Naming.** The protagonist's roster id is literally `"player"` — player-centric.
  Wants a name for what it *is* (storyline: an AI whose abilities are theorems),
  renamed before robot-as-actor hard-codes `"player"` everywhere (the
  id-matches-label rule).

## Architecture (the two ports, in detail)

### intent-in — the body as a universal intent-resolver

One resolution layer, identical for player / NPC / enemy / possessed body. Given
an `ActorControlFrame`, the body's capabilities, and its current physical state
(cooldowns, stun, locomotion mode, gravity frame), it resolves **every** intent —
move, jump, dash, fire, melee (+ tilts), blink, fly-toggle, shield, ledge-grab,
special — into effects and emits per-intent *accepted | blocked* feedback.
Movement is just another intent: the controller emits one frame-agnostic desired
body-local velocity and the body projects it to its current mode (grounded
throttle + jump edge, or free-mover velocity) — one vocabulary, two projections.
This generalizes the resolution the *player* body already has (e.g. the
fire-rate + resource gate in `ProjectileSpawner`) to every body.

### world-out — headless perception (`WorldView` + `WorldMemory`)

`WorldView` is everything in the viewport: local terrain / solids, other actors
(pos / vel / facing / disposition / body-state), projectiles (pos / vel / kind /
threat), hazards, items, portals (apertures + destinations), and self (kin +
per-capability availability + last tick's feedback). `WorldMemory` is the
per-controller belief that outlives the viewport (I6). Tactical queries ("is the
target in my line of fire?", "is it reachable?") are answered over the **same
solids the body physically collides against** — reusing the real collision query,
never a parallel sensor. Built once per body per tick, in the gameplay layer.

### the controllers and the harness

Controllers plug into both ports and are interchangeable (I1). The strong brain
(I9) reads `WorldView` + `WorldMemory` and drives the full kit. The harness (I8)
builds a minimal but real headless `App` — real `RoomGeometry` (and portals), the
real actor / aggression / projectile / integration systems — places bodies with
chosen controllers, and ticks. The current brain-policy proxy (`smash/arena.rs`,
own kinematics, no terrain) is retired in favor of it; the brain's pure-stage
unit tests stay for fast checks but stop certifying "works in a fight."

## Current state (starting point — HISTORICAL, the S0 baseline)

> This snapshot is the baseline the roadmap started from (pre-S1). For LIVE status
> read the **Progress** section above; several leaks below are already fixed (the
> brain-side fire cadence is deleted, sim-time is threaded, the actor kit is at
> parity). Kept as the "before" picture.

- **Input seam — present and correct.** `Brain` → `ActorControlFrame`
  (`ambition_characters::brain`, `actor/control.rs`); possession routes a player
  controller onto a body.
- **Body enforcement — inconsistent.** The player's fire is gated by a body-side
  cooldown + resource meter (`projectile::ProjectileSpawner`); the enemy fire path
  spawns on every intent with **no body cooldown**. Melee has a body cooldown,
  ranged does not. (A fire-rate limit currently lives in the *brain* — that is the
  leak I3 forbids, and it is deleted in S1.)
- **Perception — a point-target.** `BrainSnapshot` / `ObservationFrame` carry a
  single target position + distance and an unused `terrain` field. No viewport,
  no other-actor / projectile / portal awareness, no memory; `sim_time` is
  hardcoded `0.0` (reaction latency is therefore inert in-engine).
- **Capabilities — player-only resolution.** The enemy integrator consumes only
  `locomotion` / `velocity_target` / `jump_pressed` / `drop_through`; blink, fly,
  shield, ledge-grab, tilts, charge-fire resolve only on the player body.
- **Motor — bifurcated and frame-bound.** Grounded `locomotion` + `jump` vs aerial
  `velocity_target` are separate paths; aerial steering and blink assume `-y` is
  up (breaks under C4 / portals).

## Progress (live — Jon reads, can't ask)

Author model: Opus 4.8 (1M). Wall-clock log at the bottom.

- **S0 (harness seed)** ✅ — `crates/ambition_gameplay_core/src/features/ecs/fighter_harness.rs`.
  A real-ECS headless `App` over the *actual* fire pipeline (`emit_brain_action_messages`
  → `spawn_enemy_projectiles_from_brain_actions` → `apply_projectile_effects`),
  driving the body through the one `ActorControlFrame` seam any controller uses
  (I1). Seeds the harness that retires the `smash/arena.rs` proxy for "works in a
  fight"; it will grow to drop full bodies in a real room as S2–S5 land.
- **S1 (body owns fire-rate)** ✅ — the intent-in seam's first migrated intent.
  `IntentOutcome { Accepted | Blocked(BlockReason) }` added to `actor/control.rs`
  (the body→controller half, shaped generically for the rest of the kit).
  `ActorAttackState` gained a body-side `ranged_cooldown` + `try_fire_ranged()`
  (the ranged analogue of the melee cooldown); the enemy fire path enforces it.
  The brain-side cadence (`SmashState::ranged_cooldown_remaining` +
  `maybe_substitute_ranged`'s gate) is **deleted** — the brain attempts a shot
  every in-band tick; the body is the floor. Acceptance specs green: a 60 Hz spam
  controller fires at the body rate (2 shots / 2 s), output rate is bounded by the
  body not the attempt rate, idle controller never fires.
- **S2 (frame-agnostic motor/perception vectors)** ✅ *partial* — the Smash
  brain's vertical reasoning no longer assumes screen `-y` is up. `ObservationFrame`
  gained `down` + frame-local accessors (`up_axis` / `side_axis` / `to_target_up`
  / `to_target_side` / `self_vel_up`, oriented so screen-down gravity is
  byte-identical to the old reads); jump-to-chase, the up/down/forward melee pick,
  decide_flight, the evade-up dodge, the aerial dive/perch arc, and the ranged aim
  are all gravity-framed now (I10). Proven: the 49 existing smash tests still pass
  (vertical byte-identity, incl. the flight-health arena) + 3 new C4 tests
  (`evade_dodges_against_gravity_under_rotated_gravity`,
  `target_above_is_gravity_relative`, `aerial_perch_climbs_against_gravity`).
  **Deferred (deliberately):** (a) the grounded Walk dir still reads screen-x —
  its sideways-gravity correctness depends on the engine spine's axis convention,
  which wants verification; (b) the `locomotion`-vs-`velocity_target` vocabulary
  merge ("one body-local desired velocity, two projections") touches
  bosses/flyers/player/possession and changes feel — needs runtime verification,
  not safe to fold blind.
- **S4 (headless perception)** — *sim-time done; the world-out value now exists.*
  - *(prior step) real accumulating sim-time:* `GameplayElapsed` resource +
    `advance_gameplay_elapsed` (sums `WorldTime::scaled_dt`, freezes on pause)
    threaded into `build_enemy_brain_snapshot` (was hardcoded `0.0`). This
    **activates reaction latency in-engine** for the first time (the `obs_history`
    lookback was inert). Test `gameplay_clock_accumulates_scaled_dt`; full-app
    plugin test confirms the wiring. NOTE: in-world feel of active reaction latency
    is unverified — wants a runtime check.
  - **`WorldView` + `WorldMemory` (the world-out port)** ✅ — the headless,
    controller-neutral perception value now exists, **body-generic from day one**
    (guardrail #1). `ambition_characters::perception` owns the value
    (`WorldView { self_view, viewport, actors, projectiles, terrain }` +
    `WorldMemory`) and its **pure tactical queries** — `line_of_fire` /
    `reachable` sweep the SAME `ae::Aabb`s the body physically collides against,
    via the SAME `AabbExt::sweep_hit` parry primitive the physics step uses (no
    parallel sensor); `nearest_hostile` / `incoming_threats` are relational
    (non-player-centric). `WorldMemory` retains last-known actor positions with a
    confidence that **decays once a target leaves the viewport** (I6: pursue the
    vanished foe), dead-reckoned by last-known velocity, forgotten below a floor.
    The **builder** is in gameplay_core (`features/ecs/perception.rs`):
    `build_world_view(body: &PerceptionBody, peers, projectiles, world, relations,
    …)` takes a BODY of **any faction** — the player-robot body is built by the
    exact same call as the PCA (proven: a Player-faction view and an Enemy-faction
    view from one function; hostility resolved from `FactionRelations`, not the
    viewer's type). Terrain is clipped from the real collision `world.blocks`.
    The view also carries **portal apertures** (`PerceivedPortal` — pos / normal /
    half_extent / channel key), with `WorldView::linked_portal` resolving the
    paired exit, so S5 can route a chase across an aperture. Proven headless: 8
    perception-value tests (`ambition_characters`) + 5 builder tests
    (`gameplay_core`) — line-of-fire blocked by a real wall / clear otherwise,
    viewport clipping (actors / projectiles / portals), relational projectile
    threat, portal pairing, memory retain-then-forget. Zero render dependency (I5).
  - **Remaining S4:** **live per-tick construction wired into the actor loop**
    (deferred: `update_ecs_actors` is at the 16-param ceiling and no brain consumes
    `WorldView` until S5 — wiring it now would churn a maxed system for a dead
    consumer; the harness proves the seam). The brain *consuming*
    `WorldView`/`WorldMemory` (enriching `decide`) is S5 by design. The view value
    itself is now **feature-complete**: self + viewport + actors + projectiles +
    terrain + portals.
- **S3 (full capability parity)** — *in progress, verb by verb:*
  - **S3a blink** ✅ — blink resolves on the actor body via the SAME
    `blink::blink_target` rule the player uses, gated by `CombatCapabilities::can_blink`
    (capability) + `ActorAttackState::try_blink`/`blink_cooldown` (the I3 floor —
    binds a possessing human too). Data-driven: `EnemyArchetypeSpec.smash_can_blink`
    projects into BOTH the brain's `SmashCfg.can_blink` (attempt) and the body caps
    (enforce). PCA authors `smash_can_blink: true` → it blink-dodges a lunge.
  - **S3b fly** ✅ — the PCA is now a grounded-base HYBRID: `decide_flight`
    PREFERS grounded and flies only to cover a long traversal gap (hysteresis;
    lands to brawl). Body resolves `fly_toggle` → flips `gravity_scale`
    (capability-gated). Data: `smash_can_fly` projects into brain cfg + body caps;
    PCA authored `is_aerial: false` + `smash_can_fly: true` (peaceful hover
    preserved via the Floating catalog body). Flight is a brain *preference* (I4),
    free for now — a resource cost comes later. Also fixed an S1 interaction the
    policy exposed: a ranged poke now ADVANCES while firing (fire-while-closing)
    instead of camping at range, so the fighter stays aggressive.
  - **S3c shield** ✅ — reactive block as a body capability. `shield_held`
    (controller attempt) lands on a new body state `ActorStatus::shield_raised`,
    gated by `CombatCapabilities::can_shield`; the actor damage path negates a
    guarded hit from the faced side using the SAME `shield_blocks_hit` directional
    rule the player uses (a hit from behind still lands). Data: one `smash_can_shield`
    flag projects into BOTH `SmashCfg::can_shield` (the brain already raises the
    guard on a lunge it won't blink) and the body's enforce gate. PCA authors it.
    Proven against the REAL actor damage system: front-guard negates, lowered guard
    takes it, back hit lands.
  - **S3d dash** ✅ — dash as a body capability that RIDES the grounded spine: on
    an accepted dash the body bursts its side velocity to `max_run_speed ×
    DASH_SPEED_MULT` and opens a window during which the spine keeps the raised
    speed cap (so the burst is sustained, not decelerated — it doesn't fight the
    motor). Frame-agnostic (dash dir via `AccelerationFrame::to_world`, so it's
    never inverted under rotated gravity). Body owns the cooldown + window
    (`ActorAttackState::try_dash` / `dash_active`, the I3 floor); the brain just
    sets `dash_pressed` on its existing Dash action (a body without `can_dash`
    still closes at walk speed — graceful fallback). Data: `smash_can_dash` →
    `CombatCapabilities::can_dash`; PCA authors it. Proven against the REAL
    integration: a dash-capable body covers >1.3× the ground of a walker over the
    window; refire-gated.
    **Feel TBD in-engine** — `DASH_SPEED_MULT = 1.7`, `DASH_TIME_S = 0.18`,
    refire `0.7 s` are first-pass numbers; the mechanics are proven headless but the
    *feel* wants a runtime check (and i-frames / an instant top-speed burst rather
    than a raised-cap ramp await the deferred S2 locomotion/velocity_target merge).
  - **S3d tilts** ✅ *already covered* — directional melee (up-tilt / down-air /
    back-air) already resolves on the actor body: the brain picks the attack axis
    (`emit` sets `attack_axis`), and `update_ecs_actors` places the hitbox via
    `enemy_melee_animation_for_axis` + `attack_aabb_dir`. No new code; the verb was
    folded into the resolver when enemy melee became data-driven.
  - **S3d special + integrator de-player-casing** — *deferred (rationale):*
    *special* resolves to a body's signature, but no body authors one yet (the
    PCA's signature is a design fork with the encounter doc / the sprite agent) —
    adding one is "new ability" content (a non-goal of this unification), so the
    `special_pressed` seam stays inert until a signature is authored. *Folding the
    player `ProjectileSpawner` onto `try_fire_ranged`* (the "stop special-casing the
    player" fire path) is feel-sensitive — it touches the player's mana meter +
    charge-fire state machine, which changes player feel and wants runtime
    verification, not a blind fold. Both are tracked in the drift note below.

  Smell logged (Jon, deferred): characters should be defined by their movement
  *kit*, not by named `EnemyArchetype` rows — `dev/journals/code_smells.md`.
- **S3e (relational damage routing)** ✅ (commits 524f67d6 melee, 05a7726e
  projectile). `FactionRelations`' default now encodes the combat baseline
  (Player↔Enemy/Boss), so it is the single damage authority with zero behavior
  change. New `HitTarget::Actor` carries a pre-resolved non-player victim: an
  Enemy/Boss melee swing and an enemy projectile both scan actor hurtboxes and
  damage any body their faction is hostile to (projectiles route by the FIRER's
  real faction, looked up from the owner — so both arena directions work, which
  the binary `ProjectileFaction` couldn't). The player is now a GATED victim
  (hitbox player-loop, player-damage consumer, projectile player-loop all skip a
  hit whose attacker isn't hostile to Player) — so a spectator-arena fighter
  spares the observer. The player's OWN attacks stay universal (NPC-provoke
  intact). 8 new headless tests against the real systems; 1019 lib green. The
  arena is now mechanically possible (needs S6's robot-as-actor for the second
  combatant + a room that sets `Enemy↔Boss` hostile and clears `→ Player`).
- **S4 (headless perception)** — sim-time + the `WorldView`/`WorldMemory` value &
  body-generic builder done (above); portal awareness + live actor-loop wiring +
  brain consumption (S5) remain.
- **S5 (strong brain + spectator arena)** — pending (needs S4 + S3e + S6).
- **S6 (convergence / de-player-casing)** — pending; the slice where "done" lands.
  The player becomes an actor, the duplicated player clusters fold, the
  player-robot becomes a droppable boss archetype, and possession is wired in-game.

Calibration outcome (2026-06-26): no pivot. S3 built actor-side parity correctly;
the remaining work is **convergence debt-paydown** — see the "end state" + audit
sections up top, codified as S3e (relational damage) and S6 (de-player-casing).
The acceptance bar is the convergence (smaller/cleaner code), with the spectator
arena as the necessary-but-not-sufficient behavioral witness, and **in-game
possession the deferred payoff**.

Drift note for the next reader: the *player* fire path still uses its own
`ProjectileSpawner` (cooldown + meter); S1 unified the **enemy/AI-driven** body
path and shaped the seam. Folding the player path onto `try_fire_ranged` (and a
shared resource gate) is **S6** (de-player-casing), feel-sensitive — done behind a
differential harness with runtime verification, not blind.

## Roadmap

A dependency chain to the end state: each slice is built once and consumed by the
next, so the strong brain is written last against finished seams with no rework.
**S1–S3 are one build-out** — the intent-in resolver — migrated one intent-family
at a time, not three separate mechanisms. Every slice is shippable and proven in
the harness.

### S0 — Real-ECS headless harness

Build the drop-a-body fixture + tick helper over the real systems. Port existing
brain regressions onto it. Author the acceptance specs (below) as the contract;
they start red and each turns green when its layer lands.

*Done when:* the fixture runs headless in milliseconds; existing regressions run
on it; the acceptance specs exist and fail honestly.

### S1 — Body owns every constraint (resolver, first intent: fire-rate)

Introduce the intent-in resolver as a real layer, and migrate **fire** into it
first: a body-side weapon cooldown, the brain emits `fire` freely, and the
brain-side fire cadence is **deleted**. Shape the resolver's *attempt → accepted |
blocked* result generically so the remaining intents slot in.

*Done when:* the fire-rate spec is green — a spam controller, a tactical brain,
and a simulated human all produce the body's weapon rate, not the tick rate.

### S2 — Unified, frame-agnostic motor (resolver: movement)

Migrate movement into the resolver: one frame-agnostic desired body-local
velocity, projected to the body's mode. Remove the post-emit aerial override and
every `-y`-is-up assumption (perch / dive / blink vectors become
acceleration-frame-local).

*Done when:* a body steers correctly under all four C4 orientations and through a
portal; one mode flip switches grounded ↔ flying with no second vocabulary.

### S3 — Full capability parity, actor-side (resolver: the rest of the kit)

Migrate blink / fly / shield / dash / ledge-grab / tilts / charge-fire / special
into the resolver on the **actor** body, gated only by the body's capabilities.
Give the PCA the full kit. This builds the actor half of the shared resolver to
parity; **it does not by itself retire the player half** — that convergence is S6.

*Done when:* every verb resolves on an actor body via a body capability + the I3
floor, data-driven, proven in the harness. (blink ✅ fly ✅ shield ✅ dash ✅ tilts
✅ already-covered; special deferred — needs an authored signature, a non-goal
"new ability".)

### S3e — Relational damage routing (finish non-player-centric combat)

Targeting is already relational (`FactionRelations`); **damage is not** — melee
hitboxes and projectiles hard-code player-side ↔ enemy-side. Generalize both to
route off `FactionRelations`, and let the player be a *gated* candidate (and an
`AggressionMode` hostile to a faction, not "to the player") so a body can ignore
the observer. This is the missing half of non-player-centrism and the hard
prerequisite for the arena.

*Done when:* an Enemy-faction body damages a Boss-faction body (and vice-versa)
while a Neutral body is untouched, pinned headless; the bipartite player/enemy
branches are gone.

### S4 — Headless perception (world-out)

Build `WorldView` + `WorldMemory` entirely in the gameplay layer, no render
dependency — **for the player exactly as for any actor** (one perception path).
Thread the real `sim_time`. Line-of-fire / reachability reuse the real collision
geometry.

*Done when:* `WorldView` is constructed and asserted in the headless harness;
reaction latency is live in-engine; a body remembers a target that left its
viewport.

### S5 — The strong universal brain (+ the spectator arena as its scene)

Evolve the brain's decide stage to consume `WorldView` + `WorldMemory` and drive
the full kit on any body: never commit an attack with no line of fire over real
geometry; reposition (jump / go-around / blink / fly) instead of pushing into a
wall; pursue last-known position off-viewport; route through portals. (observe →
mode → action → emit is reused; S4 changes what `observe` reads, S5 enriches
`decide`.) The **spectator arena** — a real room where a PCA and the player-robot
are mutually hostile, the observing player Neutral, both under this brain — is the
in-engine form of the mirror-match test (the arena *test* already does this with
two PCAs); it needs S3e (relational damage) + S6 (robot-as-actor).

*Done when:* advanced-vs-advanced, no-wedge / no-OOB sweep, and portal-routing
specs are green for both the player-robot and the PCA under this one brain, and the
spectator arena is playable.

### S6 — Convergence: the player IS an actor (de-player-casing)

The slice that makes "done" real (see "what 'done' actually means"). The player
body writes an `ActorControlFrame` and runs the **one** resolver + perception path;
the player-only clusters (`PlayerShieldState` / `PlayerDashState` /
`PlayerFlightState` / `ProjectileSpawner`) are folded into shared capability state,
not mirrored. The player-robot becomes an actor archetype (renamed off `"player"`),
**droppable as a boss** (I7). **Possession is wired through for real in-game** (I2)
— the deliberately-deferred payoff. Feel-sensitive (player mana / charge / dash);
done with runtime verification, behind a differential harness, not blind.

*Done when:* the duplication in the convergence audit is provably **gone** (one
resolver, one perception path, one damage model — measurably less code); possess
the PCA → full moveset in-game; drop the player-robot as a boss → full moveset;
one code path certified in the harness AND felt in-engine.

### Guardrails — do not make the S6 convergence harder

S4 and S5 land *before* the player is folded onto the shared path (S6). The trap is
building them in a way that deepens the very player/actor split S6 has to undo.
Every later slice must move toward convergence, never away. Concrete rules:

1. **Build perception body-generic from day one.** `WorldView` / `WorldMemory` are
   functions of a **body** (any faction), constructed for the player-robot body
   exactly as for the PCA. Do NOT hang perception off the enemy-only
   `build_enemy_brain_snapshot` / `ObservationFrame` path or key it on `EnemyBrain`.
   The human player reads raw input today (no `ObservationFrame`); perception "for
   the player" means *a brain driving the player-robot body* (the arena / boss-drop)
   gets the same `WorldView` — so make construction take a body, not an "enemy."
2. **The strong brain (S5) takes a body + its `WorldView`, never an actor-only or
   enemy-only type.** It must already be drivable on the player-robot body — that
   IS the I7 / mirror-match test. If it can't drive the player body, it's the wrong
   shape.
3. **Add no new `"player"`-string couplings or `Player*`-only clusters.** New
   capability/state goes on the shared `CombatCapabilities` / `ActorAttackState` /
   `ActorStatus` vocabulary. The damage model is already relational (S3e:
   `FactionRelations` + `HitTarget::Actor`) — route through it; never reintroduce a
   player-vs-enemy branch.
4. **S3e is additive on purpose — don't "finish" it by ripping things out.** The
   player's OWN attacks stay universal (so striking a peaceful NPC provokes it);
   hazards / pogo / charge-crash / breakables are deliberately NOT faction-gated.
   Tidying these into the relational path is not the convergence and breaks
   provoke / hazards. Leave them.
5. **S6 is a checkpointed refactor, not a blind rewrite.** Get the shape right
   first; build the parity net BEFORE folding. The net is the existing trace
   tooling: `crates/ambition_gameplay_trace/` (the per-frame player feel-trace ring
   buffer + markdown dump) and `actor_trace.rs` in it (the non-player-centric OOB
   flight recorder — one `Query<&BodyKinematics>` over every body). Capture a
   player movement/combat trace before the fold and diff after. Replay/feel may
   change — only the compile + the feel diff gate it. Commit = checkpoint, then
   keep moving. (Jon verifies feel in-game; ship a feel-sensitive change blind in
   its own marked commit and ask.)

## Acceptance scenarios (what "done" means)

The behavioral scenarios below are **necessary but not sufficient**. The
sufficient condition is **convergence** — the same behaviors running on *one*
implementation, with the convergence-audit duplication gone (smaller, cleaner,
better-organized code). A scenario that passes on two parallel paths has not met
the bar.

- **Convergence (the sufficient condition)**: one intent-resolver, one perception
  path, one relational damage model, shared by the player and every actor; the
  player-only clusters retired or folded; net less code than today. This is what
  the behavioral scenarios are *evidence for*, not a substitute for.
- **Spam-equivalence** (I3): a spam controller and a human produce the same
  physical output on the same body.
- **Drop-anywhere** (I8): any body dropped at any position behaves reasonably and
  never wedges or leaves the world.
- **Spectator arena / mirror match** (I9): a PCA and the player-robot, mutually
  hostile under the one strong brain with the observing player Neutral, fight each
  other without degenerate loops and (S5) route around the observer — "have them
  fight and see what happens so we can test for degeneracies" (Jon). The in-engine
  form of the brain arena test. Necessary for acceptance, not sufficient. Doubles
  as an out-of-bounds soak test.
- **Possession + boss parity, in-game** (I2, I7): possess the PCA → full moveset;
  drop the player-robot as a boss → full moveset; one code path — and it actually
  works in-game (possession is currently unwired by choice, pending S6).
- **Frame-agnostic routing** (I10): the strong brain fights and navigates
  correctly under C4 gravity and through portals — "make sure the advanced AI
  brain routes through portals effectively" (Jon).

## Non-goals

- **Training an RL policy** — out of scope, but every port is shaped for it: one
  input seam, one observation space (`WorldView` + `WorldMemory`), body-enforced
  constraints (so a learned policy probes the real action space, degenerate lines
  included). Dropping in `Brain::RlPolicy` must require no new plumbing.
- **New abilities** beyond the existing player kit — this unifies the kit we have
  onto every body.
- **Coupling perception to rendering** — forbidden (I5); the game runs headless.

## Pointers (current as of S4 world-out value)

- Input seam: `crates/ambition_characters/src/brain/` (`mod.rs`, `smash/`),
  `actor/control.rs` (`ActorControlFrame`; the body→controller half is
  `IntentOutcome` / `BlockReason`).
- Body enforcement (the resolver, actor side): `features/enemies/integration.rs`
  (`ActorMut::update` — grounded spine + dash burst), `features/ecs/actors/update.rs`
  (`update_ecs_actors` — resolves blink/fly/shield/dash AFTER `em.update()`),
  `combat/components/actors.rs` (`ActorAttackState` — per-verb cooldowns +
  `try_fire_ranged`/`try_blink`/`try_dash`), `combat/components/mod.rs`
  (`CombatCapabilities` — the per-body kit gate), `features/ecs/brain_effects.rs`
  (the enemy fire body-gate — DONE in S1).
- The player-side resolution still to FOLD (S6): `projectile/spawn.rs`
  (`ProjectileSpawner`, cooldown + mana meter) and the player ability clusters
  (`PlayerShieldState` / `PlayerDashState` / `PlayerFlightState`) in
  `ambition_engine_core/src/movement/`. Shared core math the fold reuses:
  `abilities/traversal/blink.rs::blink_target`, `combat/damage.rs::shield_blocks_hit`,
  `ambition_engine_core::integrate_normal_spine`.
- Relational damage (S3e): `combat/targeting.rs` (`FactionRelations` — the matrix +
  `select_actor_targets`), `combat/events.rs` (`HitTarget::Actor`), `combat/hitbox/mod.rs`
  (melee), `projectile/systems.rs` + `enemy_projectile/systems.rs` (projectiles),
  `features/ecs/damage/` (the actor victim consumer), `combat/damage.rs` (the player
  victim consumer + gate).
- Perception (S4): **the world-out value** is `ambition_characters::perception`
  (`WorldView` / `WorldMemory` / `SelfView` / `PerceivedActor|Projectile|Solid` +
  the pure `line_of_fire` / `reachable` / `nearest_hostile` queries); **the
  body-generic builder** is `features/ecs/perception.rs`
  (`build_world_view(body: &PerceptionBody, …)` — takes a body of any faction,
  guardrail #1, NOT the `enemy`-named path). Construction reads `RoomGeometry`
  (clip `world.blocks` to the viewport), `FactionRelations` (relational hostility),
  and `features/mod.rs::GameplayElapsed` (the accumulating sim-time). The legacy
  enemy snapshot is still `build_enemy_brain_snapshot` (`features/ecs/actors/update.rs`)
  + `BrainSnapshot` / `ObservationFrame` (`brain/`) — the brain reads it today; S5
  moves the brain onto `WorldView`. Remaining: `portal/` awareness in the view +
  live per-tick wiring into `update_ecs_actors` (at the 16-param ceiling).
- Possession: `abilities/traversal/possession` + the possessed branch in
  `update_ecs_actors` (player input → the body's `ActorControlFrame`). NOTE:
  possession is NOT wired through in-game yet — deferred to S6 by choice.
- Harness model: minimal-plugin Bevy `App` + manual `app.update()` + world asserts
  (`features/.../tests`, `combat/hitbox/tests.rs`, `enemy_projectile/systems.rs` tests,
  `features/ecs/fighter_harness.rs`). Runs in ms.
- Exemplar character + encounter: `docs/planning/perfect-cellular-automaton-encounter.md`.

## Implementation notes & gotchas (a memory-less agent will hit these)

Hard-won this build-out; not obvious from the code. (Repo-wide rules — relativity,
elegance, crate split, commit style — are in `AGENTS.md`; this is the slice-specific
layer.)

- **Build / test target.** `~/.cargo/bin/cargo`. The fast inner loop is
  `cargo test -p ambition_gameplay_core --lib` (~20s, 1019 tests), plus
  `-p ambition_characters --lib` and `-p ambition_content`. The full
  `ambition_app` build needs system libs and is ~10 min — design for incremental
  rebuilds; don't churn `lib.rs` or put `Reflect` on hot paths.
- **Pre-existing unrelated failure:** `ambition_characters` lib test
  `brain::player::tests::blink_precision_aim_is_screen_relative_by_default_quick_is_locomotion`
  fails at baseline — NOT introduced by this work. Everything else is green.
- **Bevy system-param ceiling (~16).** `update_ecs_actors` and `step_projectiles`
  are AT it. To add a `Res`/`Query`, bundle several into one tuple param
  (`(world_time, sim_clock): (Res<..>, Res<..>)`) rather than adding a slot.
- **Bevy `.chain()` length ceiling (~17).** When a system chain is full, register
  the extra system separately with an explicit `.before(...)`/`.after(...)` edge.
- **`Block::solid(name, min_corner, size)` takes the MIN corner, not the center;**
  `ae::World::new(name, size, spawn, blocks)` adds NO implicit boundary walls. Test
  worlds must author their own floor/walls.
- **Sim-time, not wall-time (ADR 0010/0011).** Perception/cooldown timers read
  `WorldTime::scaled_dt` (or the accumulated `GameplayElapsed`) so bullet-time and
  pause compose for free; `raw_dt` only for genuinely real-time things. S4's
  reaction-latency lookback already rides `GameplayElapsed` as the snapshot
  `sim_time`.
- **Bevy 0.18 Message API.** Buffered events are `Message` (not `Event`):
  `MessageReader` / `MessageWriter` / `app.add_message::<T>()`. The old `Event` is
  observer-only now. S4/S5 will add message types — use the Message API.
- **Determinism:** Bevy `Query` iteration order is NOT stable; sort any
  order-sensitive pass by a stable id (e.g. `owner_id`/`config.id`), not `Entity`.
- **Style/formatting hazard (UNRESOLVED — see note to Jon):** the working tree is
  not clean under the current `rustfmt`, so `cargo fmt -p <crate>` and
  `rustfmt <file>` (it follows `mod` decls) reflow whole unrelated files/trees.
  Until a toolchain is pinned, match surrounding style by hand, stage explicit
  paths, and `git diff` to confirm only your lines changed. (This contradicts
  `AGENTS.md` "Style", which says to run `cargo fmt` — flagged for Jon to reconcile.)
- **Unverified feel:** the dash tuning (`ActorAttackState::DASH_SPEED_MULT = 1.7`,
  `DASH_TIME_S = 0.18`, refire `0.7s`) is headless-proven but NOT feel-checked.
  Jon verifies feel in-game (he can't see agent-side GUIs). Ship a visual/feel/
  unreproducible fix BLIND in its own marked commit and ask him to verify — round
  trips are expensive, reverts are cheap. Don't speculate a "looks right" change as
  done.

## Wall-clock log

- S0 harness seed + S1 body-owns-fire-rate: 2026-06-26 (one session). Recon
  (parallel code map) → IntentOutcome seam + body ranged cooldown → delete brain
  cadence → real-ECS harness + 4 acceptance specs → green. Commit `b4039987`.
- S2 frame-agnostic vectors (partial): same session. Commit `8a3541c6`.
- S4 sim-time threading (first step): same session. Commit `518838df`.
- S4 `WorldView` + `WorldMemory` value & body-generic builder: 2026-06-26. Recon
  (two parallel code-map agents: collision geometry + actor/projectile/portal data)
  → `ambition_characters::perception` (value + pure line-of-fire/reachability/memory)
  → `features/ecs/perception.rs` builder (body-generic, relational) → 11 headless
  tests green; 1023 gameplay_core lib + 229 characters lib (the 1 pre-existing
  blink-aim failure unchanged).
