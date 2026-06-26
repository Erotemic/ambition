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

## Current state (starting point)

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
- **S2–S5** — pending (see below). Next: unify the motor (movement into the
  resolver, frame-agnostic).

Drift note for the next reader: the *player* fire path still uses its own
`ProjectileSpawner` (cooldown + meter); S1 unified the **enemy/AI-driven** body
path and shaped the seam. Folding the player path onto `try_fire_ranged` (and a
shared resource gate) is part of the S3 "stop special-casing the player" work.

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

### S3 — Full capability parity (resolver: the rest of the kit)

Migrate blink / fly / shield / dash / ledge-grab / tilts / charge-fire / special
into the resolver, gated only by the body's capabilities. Give the PCA the full
kit. The integrator stops special-casing the player.

*Done when:* the player-robot dropped as an AI boss wants for nothing, and the
player possessing the PCA has its full moveset — one code path, both pinned in
the harness.

### S4 — Headless perception (world-out)

Build `WorldView` + `WorldMemory` entirely in the gameplay layer, no render
dependency. Thread the real `sim_time`. Line-of-fire / reachability reuse the real
collision geometry.

*Done when:* `WorldView` is constructed and asserted in the headless harness;
reaction latency is live in-engine; a body remembers a target that left its
viewport.

### S5 — The strong universal brain

Evolve the brain's decide stage to consume `WorldView` + `WorldMemory` and drive
the full kit on any body: never commit an attack with no line of fire over real
geometry; reposition (jump / go-around / blink / fly) instead of pushing into a
wall; pursue last-known position off-viewport; route through portals. (The brain's
existing observe → mode → action → emit stages are reused; S4 changes what
`observe` reads, S5 enriches what `decide` does.)

*Done when:* the advanced-vs-advanced, no-wedge / no-OOB sweep, and portal-routing
specs are green for both the player-robot and the PCA under this one brain.

## Acceptance scenarios (what "done" means)

These are authored red in S0 and define completion:

- **Spam-equivalence** (I3): a spam controller and a human produce the same
  physical output on the same body.
- **Drop-anywhere** (I8): any body dropped at any position behaves reasonably and
  never wedges or leaves the world.
- **Mirror match** (I9): one strong brain fights itself across the player-robot
  and the PCA without degenerate loops — "have them fight and see what happens so
  we can test for degeneracies and use the feedback to refine" (Jon). Doubles as
  an out-of-bounds soak test.
- **Possession + boss parity** (I2, I7): possess the PCA → full moveset; drop the
  player-robot as a boss → full moveset; one code path.
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

## Pointers

- Input seam: `crates/ambition_characters/src/brain/` (`mod.rs`, `smash/`),
  `actor/control.rs` (`ActorControlFrame`).
- Body enforcement: `crates/ambition_gameplay_core/src/projectile/spawn.rs`
  (`ProjectileSpawner` — the player-side resolution to generalize),
  `features/enemies/integration.rs` (what an enemy body consumes today),
  `features/ecs/brain_effects.rs` (the enemy fire path needing a body gate).
- Perception: `build_enemy_brain_snapshot` (`features/ecs/actors/update.rs`),
  `BrainSnapshot` / `ObservationFrame` (`brain/`), `RoomGeometry`, `portal/`.
- Capabilities: the player ability clusters (blink / fly / shield / ledge / dash)
  and `abilities/traversal/possession`.
- Harness model: the Bevy world-assert pattern under `features/.../tests`;
  `RoomGeometry` construction in `enemy_projectile/systems.rs` tests.
- Exemplar character + encounter: `docs/planning/perfect-cellular-automaton-encounter.md`.

## Wall-clock log

- S0 harness seed + S1 body-owns-fire-rate: 2026-06-26 (one session). Recon
  (parallel code map) → IntentOutcome seam + body ranged cooldown → delete brain
  cadence → real-ECS harness + 4 acceptance specs → green. Commit `b4039987`.
