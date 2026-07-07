# Multi-limb bosses — coordinator + driven limbs

> **⚠ DRAFT — REFRAMED 2026-07-05 (do not execute from this doc).** Jon's
> direction now makes GNU-ton a **mounted boss**: the giant gnu is the MOUNT
> (carrying the multi-limb rig), gnuton is the RIDER whose brain drives it
> through the ADR 0020 `ControlGrant`. The coordinator below is therefore the
> **rider's brain via the mount grant**, not the head-core — and the "one
> genuinely new mechanism" this draft flagged (a coordinator writing other
> entities' `ActorControl`) has since LANDED as `steer_mount_from_rider`
> (mount C1); the limb rig generalizes it 1→N. The analysis and research
> findings below remain valid and are absorbed by
> [`../../reviews/fable-review-2026-07-05.md`](../../reviews/fable-review-2026-07-05.md)
> **AJ12 + R10** — execute from there. Extends
> [`boss-system.md`](boss-system.md).

The goal is **expressive bosses**, not preserving any current boss's feel. GNU-ton in
particular has no gameplay feel worth conserving; this refactor exists to *add
expression* by giving each part real, independently-choreographed motion.

---

## The question

Should GNU-ton's hands be their own actor? Yes — but as **coordinated limbs (own
body, shared brain)**, not independent thinkers. By the actors-vs-props taxonomy the
hands have no brain of their own; they do what the boss brain choreographs. So the
elegant unit is a *driven body* the brain commands via the existing control seam, not
a new AI.

## What's weird about GNU-ton today (the diagnosis)

GNU-ton is a **multi-body actor wearing a single-entity costume**. Every oddity is a
symptom of forcing a many-bodied thing into one entity:

1. **Stationary backdrop giant.** Its `BodyKinematics` barely moves (`StationaryGiant`
   + tiny sway). Every other boss's *body is the fighter* that travels the shared
   aerial seam. GNU-ton's body is set dressing.
2. **Hands move by sprite animation, not the shared kinematic seam.** For every other
   actor, motion = `BodyKinematics` integrated through the movement limb → real sim,
   headless-testable, frame-agnostic. GNU-ton's hand motion is baked into per-frame
   sprite RON (`left_hand`/`right_hand` parts tracked frame-by-frame) — **animation,
   not simulation.** This is the core relativity-principle violation.
3. **`body_damage: 0`.** The body is non-interactive; the only hurtbox is the `head`,
   the only hitboxes are hands/head-descent/shockwave/apples.
4. **Split-layer rendering** (`_body` behind platforms + `_hands` child overlay in
   front of the player, kept in lockstep by `sync_boss_split_overlay`). Unique special
   case.
5. **Per-frame multi-part hit geometry** — a much heavier authoring burden than the
   static `StrikeRect` table other bosses use.

## The reframe (thesis)

**Every part is a driven body on equal footing; one brain choreographs them all.**

- The **head-core** carries the boss identity: `BossConfig`, `BossEncounter`,
  `BodyHealth` (aggregate HP), the hurtbox, and the `Brain::StateMachine(BossPattern)`
  that choreographs everyone. It is a coordinator that *also* happens to be a visible
  body — **not** a stationary anchor. The head may move like any limb; a mobile head
  is native, not a contradiction to design around.
- The **hands** become ordinary simulated actor bodies — **no `Brain`, no
  `BossConfig`, no `BodyHealth`** — carrying `ActorControl` + `ActorMoveset`, steered
  by the head-core's brain writing their control frames each tick.
- Each tick the brain produces a **per-limb set of intents** for `{head, left_hand,
  right_hand}`: a `velocity_target` (a real-sim trajectory) plus attack edges, fanned
  out into each body's `ActorControl`.

Improving the boss then = adding limbs or richer per-limb motion, never fighting the
abstraction. `HeadDescent` is just the head-core steering itself down and striking.

## Why this is cheap: the downstream sim is already N-body-safe

Research finding worth preserving — the seam cleanly separates *produce frame* from
*consume frame* (the `ActorControl` component), and everything downstream operates
per-entity with **no 1-brain-1-body assumption**:

- `integrate_sim_bodies` / `integrate_actor_body`
  (`ambition_actors/src/features/ecs/actors/update.rs:662`, `:522`) reads each
  body's own `control.0` and drives the shared engine seam.
- `advance_move_playback` (`ambition_actors/src/combat/moveset.rs:299`) spawns
  a `Hitbox` with `anchor: HitboxAnchor::FollowOwner` — **re-resolves the AABB from the
  owner entity's position every tick** (`combat/hitbox/mod.rs:23`), so a hand's strike
  is correct with zero new plumbing.
- `apply_hitbox_damage` (`combat/hitbox/mod.rs:57`) attributes via `Hitbox.owner:
  Entity` — already per-body.
- `trigger_moveset_moves` (`moveset.rs:450`) fires a body's move from *its own*
  `control.0.melee_pressed` / `special_pressed` / `fire`.

So a hand = an actor body **without** a `Brain`; the coordinator writes its
`ActorControl` directly. `tick_actor_brains` skips it (no `Brain` → not in that query);
integration + moveset trigger + hitbox + damage all pick it up unchanged.

### Hard-coded 1-brain-1-body assumptions to respect

- **`ControlledSubject` single-subject invariant**
  (`abilities/traversal/possession.rs:71`, `debug_assert!(count <= 1)`): only ONE
  entity may carry `Brain::Player(PRIMARY)`. Keep the brain on the head-core; fan
  frames *out* to limbs — never give limbs the player brain.
- **Brain + `ActorControl` co-located per entity** — every driver query assumes a
  brain's output lands on the same entity. The coordinator must write into *other*
  entities' `ActorControl` (via `Commands`/relationship query); no current system does
  this — it's the one genuinely new mechanism.
- **`BossAttackIntent`/`BossAttackState`/`MovePlayback` are per-entity singletons** (a
  boss plays one move at a time, `Without<MovePlayback>` gate). Two hands attacking at
  once ⇒ two entities — which is exactly this plan.
- **No gameplay body-ownership relationship exists.** Closest template is the
  mount link: `MountSlot { rider: Option<Entity> }` / `RidingOn { mount: Entity }`
  (`features/ecs/mount/mod.rs:66,79`) — bidirectional `Entity` refs with per-tick sync.

## Plan (phases)

No regression net — GNU-ton's current feel is not worth preserving; the gate is
"compiles + drives the real sim," and the payoff is added expression.

1. **Limb bodies + relationship.** Add a `MountSlot`/`RidingOn`-style bidirectional
   link (`BossLimbs { hands }` ↔ `BossLimb { core }`). At spawn, create two hand
   bodies via the `boss_actor_cluster` recipe (`spawn_actors.rs:542`): gravity-free
   `flight_direct_velocity` movers with `ActorControl` + `ActorMoveset`, **no `Brain`,
   no `BossConfig`, no `BodyHealth`**. Head-core = the existing boss entity, now a free
   mover (drop the stationary assumption).
2. **Per-limb choreography — the expression work.** The scripted `BossPattern`
   (`ambition_characters/src/brain/boss_pattern/`) maps each attack onto per-limb
   `velocity_target` + attack edges and writes each limb's `ActorControl`. This is
   where new expression lives: independently-timed hand arcs, converging/diverging
   motion, a lunging head, sweeps that actually travel through the sim. Sort limbs by
   stable id for determinism. `apple_rain` stays coordinator-owned.
3. **Hitboxes from bodies.** Each limb's moveset move spawns `FollowOwner` hitboxes
   anchored to that limb. Delete the per-frame `left_hand`/`right_hand` parts from
   `gnu_ton_boss_spritesheet.ron` and the `HAND_SLAM`/`HAND_SWEEP` `StrikeRect` tables
   in `boss_encounter/attack_geometry/`; author strike geometry body-local on each
   limb's move.
4. **Rendering as actors.** Hands render as normal actors from the `gnu_ton_hands`
   sheet with their own animator and z (overlay layer). Delete `sync_boss_split_overlay`,
   the `BossOverlayLayer` child, and `BOSS_SPLIT_OVERLAY_Z`
   (`ambition_render/src/rendering/actors/boss.rs`). Revisit the head-core's z rule now
   that it moves.
5. **Land it, don't over-generalize.** Clean up the dead single-entity multi-part
   machinery where GNU-ton was the only user. Note the "coordinator + driven limbs"
   shape here, but hold off on a formal reusable API until a *second* multi-limb boss
   lands (the "add knobs when use cases land" rule). Then check it against the engine
   oracle: *could a tentacle/turret boss be added by a content-crate row without
   editing core?*

## Knobs left open (cheap later, not now)

- **Destructible / damageable limbs** — routing a hurtbox hit on a limb to the
  coordinator's HP (or the limb's own `BodyHealth`) is a small future add; the
  limb-body structure makes it natural. Not building it yet.
- **A non-body coordinator** — if a future boss's identity shouldn't live on any
  visible part (a swarm), promote the coordinator to its own entity then.
  Head-as-coordinator is the right generality for GNU-ton now.

## Pointers

- Control seam: `ambition_characters/src/brain/mod.rs:124` (`Brain`), `:160`
  (`Brain::tick`); `ambition_characters/src/actor/control.rs:151` (`ActorControlFrame`);
  `ActorControl` component at `brain/mod.rs:259`.
- Boss tick / fan-out model: `features/ecs/bosses/tick.rs:263`
  (`tick_boss_brains_system`), `:105` (`trigger_boss_attack_moves`), `:16`
  (`mirror_intent`); scripted cursor in `brain/boss_pattern/tick.rs:115`
  (`advance_scripted`).
- Spawn recipe: `features/ecs/spawn_actors.rs:542` (`boss_actor_cluster`), `:619`
  (`spawn_boss_with_overrides`).
- Content: `assets/data/boss_profiles.ron` (`gnu_ton` row ~`:278`),
  `boss_encounters/gnu_ton.ron`, `assets/sprites/gnu_ton_boss/gnu_ton_boss_spritesheet.ron`
  (the per-frame hand parts to delete).
- Render split: `ambition_render/src/rendering/actors/boss.rs` (`sync_boss_split_overlay`
  `:198`, split z-consts `:39`,`:44`).

## Status

Design only — nothing implemented. Open decisions above are unresolved. Next concrete
step when picked up: Phase 1 (limb relationship + spawning the hand bodies), then
Phase 2 to prove per-limb fan-out before deleting the old geometry.
