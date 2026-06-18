# ADR 0018: Enemy cluster variation

## Status

Accepted.

## Context

Multiple enemies of the same archetype, spawned at the same instant in the
same room, share their brain config and per-actor state. With no
randomization that means:

- Every `Skirmisher` ticks the same `cooldown_remaining` by the same `dt`
  and fires on the same beat — a squadron of shark-riders volleys in
  unison.
- Every `MeleeBrute` computes the same chase vector toward the same
  target and walks the same speed — they overlap into a single
  stomping shape and clip each other's hitboxes.
- Aerial actors pick the same orbit offset and stack at one axis.

The user's complaint that triggered this ADR: *"shoot in unison",
"clump up", "look like one big enemy."* The previous fix for the
`enemy_default_brain` `Skirmisher` path applied per-actor jitter
(cooldown, initial stagger, standoff, orbital phase, drift); when the
brain construction was duplicated into a parallel
`spawn_composite_mount_rider` path that knowledge was lost, and the
unison reappeared. That regression is the proximate trigger, but the
underlying problem is broader: **every brain path that spawns a hostile
must vary the actor noticeably from its peers, and the rule has to be
visible enough that the next person duplicating a brain config doesn't
silently re-introduce uniformity.**

We also want room to grow beyond "tick jitter" into deliberate goal /
plan variation: a pirate that decides to retreat to position X is more
interesting than one that always advances to attack_range. The
brain-frame design (`Brain::tick_with_actions` → `ActorControlFrame`)
already supports this — the brain is free to emit any intent each
tick, including "move toward an authored offset point." This ADR
codifies the policy so it survives future brain additions.

## Decision

### 1. Every brain-spawn site applies per-actor jitter.

Every place that constructs a brain config from an `EnemyRuntime`
must derive a stable per-actor seed and apply at minimum:

- **Cadence jitter** (any cooldown / interval): ±25%.
- **Initial stagger** (initial `cooldown_remaining`): random fraction
  of the post-jitter cadence, in `[0.3, 1.0]` of the cooldown so a
  fresh group's first volley spreads across roughly a full beat.
- **Spatial offset jitter** (orbit phase, retreat goal, standoff
  radius): per-actor offset around the authored target geometry.
  ±20% on radii, full `[0, τ)` on orbital phase.

The seed is `crates/ambition_gameplay_core/src/combat/variation.rs::seed_from_id(&enemy.id)`,
which is stable across runs (deterministic) but distinct per
authored `EnemyRuntime.id`. Composite spawn paths use the rider /
mount sub-id (e.g. `"<authored>:rider"`) so a fan-out doesn't
collapse two children to the same seed.

The canonical helper is `five_f32s_from_seed(seed) -> (f32, f32,
f32, f32, f32)` in `crates/ambition_gameplay_core/src/combat/variation.rs`: an xorshift32
sequence producing five independent uniforms in `[0, 1)`. Per-brain
config decides which jitter dimensions consume which slot.

### 2. Jitter is uniform, not derived from archetype identity.

Jitter dimensions are intentionally generic — the same `±25%
cadence` rule applies to a Skirmisher's fire cooldown and a
MeleeBrute's attack cooldown. Per-archetype "this brute is wild and
shouldn't jitter" exceptions are forbidden; that's a brain-template
choice, not a per-archetype knob, and would defeat the
uniformity-of-rule that makes the policy auditable.

### 3. A duplicated brain-config call site MUST also duplicate the jitter.

If a system constructs a `SkirmisherCfg { ... }` or `MeleeBruteCfg
{ ... }` literal anywhere other than the `enemy_default_brain` path
(see `spawn_composite_mount_rider`, `enforce_mount_rider_link`'s
dissolve branch, future scripted spawns), the call site MUST also
apply per-actor jitter. The policy is comment-tagged at each such
site: `// Per-actor jitter — see ADR 0018.`

Failure mode this avoids: a contributor copies the inline cfg from
one spawn path into a new one and forgets the jitter. The pattern
where unison-fire regressed (the original
`spawn_composite_mount_rider`) is exactly this failure.

### 4. Goal-and-plan variation is layered in, not hardcoded.

The current `MeleeBrute` brain emits chase-toward-target every tick.
This is fine for grunts but reads as "same-shaped mob" when three
brutes spawn together. The next-step shape (not blocking on this
ADR, but the design target it enables):

- A brain may carry a **per-actor goal** — a position, a target id,
  or a behavior verb (Retreat, Flank, Hold). Goals are seeded from
  the same per-actor RNG; some are randomized, some are chosen by
  the encounter (boss says "you three flank, you two charge").
- Each tick the brain emits intent **toward the goal**, not toward
  the player directly. Goals are reassessed on a slow clock (1–3 s)
  or on triggering events (took damage, ally died, lost line of
  sight).
- Action selection inside a goal still has per-actor jitter (which
  attack to throw, which dodge to use) — orthogonal to goal
  selection.

The implementation hook is `BrainSnapshot` + `ActorControlFrame`:
the snapshot grows a `goal: Option<Goal>` field; the brain reads it
and writes intent accordingly. No brain-template multiplication is
required; existing templates (Patrol, MeleeBrute, Skirmisher) become
goal-aware in place.

This ADR DOES NOT require goal/plan support to be implemented yet.
It documents the direction so the jitter-everywhere rule isn't
mistaken for the end state.

### 5. Visual / hitbox crowding is a separate problem, with its own knob.

`SmashCfg.crowding_threshold` already exists and the `MeleeBrute` /
`Skirmisher` paths can grow the same. The brain reads
`snapshot.crowding` and uses it to bias movement away from peers.
This ADR does not require crowding to be wired into every template
today, but it forbids "fix overlap with a hack" — overlap avoidance
goes through the crowding signal, not through brain-internal
heuristics that the rest of the system can't observe.

## Consequences

- Every new brain template must specify which jitter dimensions
  apply at construction. The `enemy_default_brain` function in
  `crates/ambition_gameplay_core/src/features/ecs/brain_builders.rs` is the canonical reference: every
  template that has parameters worth jittering does so there.
- Tests for "deterministic brain tick given fixed snapshot" still
  hold — the jitter is per-actor, not per-tick, and the seed is
  stable across runs.
- The `crates/ambition_gameplay_core/src/combat/variation.rs` helpers are the single jitter source. New
  brains that need more than five slots either (a) reduce, (b)
  reuse with a second offset seed, or (c) extend the helper. Do not
  introduce a parallel jitter source.
- The "two brain construction sites" pattern (default + composite
  fan-out) is a permanent footgun. Future refactors should DRY it
  by passing a `BrainModeContext` through a single builder; until
  then the comment marker is the gate.

## Current implications for agents

When constructing hostile actor brains, do not hand-roll a parallel source of
per-actor randomness. Use the shared variation helpers in
`crates/ambition_gameplay_core/src/combat/variation.rs`, and keep
mount/rider fan-out, dismount, encounter-spawn, and default enemy-brain paths
on the same seeding policy. A new brain construction path should make its
variation choice explicit in the same place it builds the `Brain` and
`ActionSet`.

## Cross-references

- ADR 0016 (actor unification) — the brain frame seam this builds
  on.
- `feedback_design_balance` memory — "narrow specific types beat
  wide generic ones; add knobs when use cases land". This ADR is
  the canonical jitter knob; future variation adds more knobs
  rather than baking variation into archetypes.
- `crates/ambition_gameplay_core/src/combat/variation.rs`
  — the canonical seed + jitter helpers.
- `crates/ambition_gameplay_core/src/features/ecs/mount.rs::enforce_mount_rider_link`
  — dissolve path that applies the rule for dismounted riders.
