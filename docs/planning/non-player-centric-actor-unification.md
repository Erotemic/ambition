# Non-player-centric actor unification — autonomous run guide

**North star:** every character — player, friendly NPC, enemy, boss — is a
different *instance* of one actor system. The only genuinely player-specific
things are: input from devices (already abstracted through the input layer),
the camera, and the HUD/UI. Everything below that — body, movement, abilities,
rendering, combat — is shared machinery an actor opts into.

This document is the self-guide for an autonomous run through all stages. No
review is needed between stages; Jon reviews the cumulative change on `main`.
Operate under `docs/concepts/autonomous-decision-making.md` (architecture forks
are mine; act, record the reasoning in the commit). Work on `main`, one coherent
commit per step (his standing rule). Build-green is the hard gate; replay is a
*tool*, used here specifically to guard the PLAYER's identity (see Methodology).

## The chosen architecture (the end-state)

**A shared physics SPINE, with abilities as opt-in COMPONENT+SYSTEM limbs on a
fixed phase pipeline.** Not "one fat function gated by flags" (closed, fat
archetypes, hard to pare down); not "free-for-all composable functions"
(ordering chaos). The synthesis:

- **Shared spine** every body runs: `Intent → ModeSelect → Integrate → Sweep →
  Resolve`. Small, no ability conditionals. This is the natural physics order;
  it is sequenced ONCE as `SystemSet`s, not re-derived per actor.
- **Abilities = a component + a system**, each registered into a spine phase.
  An ability is *present iff the entity carries its component*. ECS gives
  pay-for-use for free: a slug isn't in the `Dash`/`Flight`/`Blink` queries at
  all — not "branches that evaluate false," but iterations that never happen.
- **Per-actor tuning** is a component (run speed, jump height, accel), not a
  flag. "Skill" = `tuning + which ability components`. A weaker actor runs
  slower (tuning) and lacks wall-climb (no `WallClimb` component); a slug has
  abilities the player never will (surface-crawl, dive-bomb) as components the
  player doesn't carry — the ability set is OPEN, and content abilities can live
  in content crates.
- **The player is just an actor**: the most ability components + a device-bound
  brain + the camera target + the HUD source. Possession = rebind the device to
  another actor. A scripted enemy = a tiny archetype + a `ScriptedPath` system,
  never in the rich queries.

This resolves Jon's three design concerns directly:
1. **Capability variation (less AND more than player):** tuning-as-data handles
   "less skilled"; open ability components handle "abilities the player lacks."
2. **Efficiency:** composition removes the real costs (fat archetypes + branches)
   — simple actors carry small archetypes and are absent from rich systems'
   queries. Composition is MORE efficient for dumb actors, not less.
3. **Paring down is free, not implemented:** a scripted enemy is optimized by
   construction (small archetype, few systems); a game without possession just
   omits the plugin. Richness is opt-in per-entity (components) and per-game
   (plugins). This is the "build another platformer by ADDING a content crate"
   oracle, satisfied in both directions (add richness / strip it).

Discipline: **shared spine, composable limbs.** Do NOT over-decompose the common
core (gravity/integrate/collide stays one path) — only the divergent verbs
(dash/fly/blink/wall-climb/ledge/pogo/dive-bomb/surface-crawl) become components.

## Where we are now (the four divergent stacks below the unified brain seam)

The universal-brain work unified the TOP (every actor emits an
`ActorControlFrame`). Below it, four forks remain:
- **Body:** `PlayerClusterQueryData` (18 rich clusters) vs `EnemyClusterQueryData`
  vs `NpcClusterQueryData` vs `BossClusterQueryData`.
- **Movement:** `update_player_with_tuning_clusters` vs `integrate_standard_enemy_body`
  vs `NpcRuntime::integrate_velocity`/`integrate_velocity_aerial` vs boss `integrate_body`.
- **Rendering:** `animate_player` + `With<PlayerVisual>` vs `animate_characters` +
  `FeatureVisual` (catalog sprites).
- **Player-only systems:** the ~8 `single_mut()` player sim systems in
  `app/sim_systems.rs` + `player_tick.rs`, entangled with global concerns
  (world clock, moving-platform advance, camera, sandbox reset).

The `PlayerClone` (`app/player_clone.rs`) is the proof the player movement stack
already drives a non-player; this run promotes everyone onto that stack the right
(composable) way and pares the player down to input+camera+HUD.

## Methodology / verification

- **Replay fixture = the PLAYER's byte-identity guard.** Through the structural
  stages the player stays byte-identical (replay green) so the giant change reads
  as "same feel, new shape." (Unless Jon opts into opportunistic feel improvement
  — see Decisions.)
- **Non-player behavior WILL change** when enemies/NPCs/bosses move onto the
  spine — that is expected and usually an improvement (decision doc). Replay
  cannot guard them (the fixture's enemies diverge); instead pin NEW intended
  behavior with focused tests, and note notable feel changes in the run log for
  Jon's review.
- **Growing "capability matrix" test:** each ability fires for actors that carry
  its component and NOT for those that don't (the open/pay-for-use invariant).
- **`cargo build --workspace` is the hard gate.** Plus the clone test + the
  per-actor parity tests stay green.
- **Run log:** keep `docs/planning/non-player-centric-run-log.md` live (per-stage
  progress, decisions, notable behavior changes) so Jon can step away and review
  the trail.

## Stages (each = a commit or few on `main`)

**Stage 0 — Vertical slice (de-risk the pattern).** Define the spine `SystemSet`
phases. Extract ONE verb — `dash` — out of the player movement monolith into a
`Dash` component + a phased `apply_dash` system. Run it on the player AND the
clone. Replay byte-identical. Proves spine/limb split + ordering before
committing to the full decomposition.

**Stage 1 — Decompose the player monolith.** Peel each remaining verb (run,
jump, fly, blink, wall-climb, ledge, pogo, dodge, shield, fast-fall,
drop-through) into component+system pairs slotted into the pipeline; delete the
monolith body as each piece leaves. Player byte-identical throughout. The hardest
stage: the implicit ordering inside `update_player_*_with_clusters` becomes
explicit phase placement. A mid-refactor compile error or feel shift is the
middle of the work — fix forward, replay catches ordering bugs.

**Stage 2 — One body type.** A shared actor body = `Kinematics + SurfaceState +
MovementTuning(component) + ability components`. Make the player an instance.
Collapse the divergent cluster query-datas toward one (or "body + ability
components"). Mechanical, compiler-driven; many query sites.

**Stage 3 — Enemies + NPCs onto the spine.** Give them the shared body +
restricted ability components + per-actor tuning; route through the phased
pipeline. Delete `integrate_standard_enemy_body` and the NPC integrators. Author
the slug's surface-crawl and the parrot's dive-bomb as CONTENT ability components
— the first proof of the open/composable model. Pin new behavior with tests.

**Stage 4 — One rendering.** Collapse `animate_player`/`PlayerVisual` into the
catalog/`FeatureVisual` path; the player renders via its `"player"` catalog entry
(preserve its exact visual: sprite tuning, anchor, `PlayerSpriteBaseline`).
Delete the player-only render path.

**Stage 5 — Bosses onto the spine.** Fold the boss body+movement onto the spine;
keep the genuinely-special parts (the `BossPattern` brain, multi-volume
hurtboxes, encounter phases) as components/abilities — don't force-fit them.
(Bosses are the most divergent; do them last.)

**Stage 6 — Peel the player to input + camera + HUD (the payoff).** Remove every
remaining player-specific system except: device→`ControlFrame` input,
camera-follow, HUD. Generalize the `single_mut()` player sim systems to iterate
player-bodied entities; move the global concerns (clock/platform/camera/reset)
into primary-only systems (the P9' decoupling). "Player" becomes a tag:
device-bound + camera-target + HUD-source. Possession = retag. The clone stops
needing a bespoke driver — it just runs the shared loop.

**Stage 7 — Combat unification (conditional).** Make attacks/projectile-charge/
ActionSet effects actor-generic too (fix the `brain.is_player()` projectile-charge
gate; `desired_vel` axis-vs-velocity split, P10). Conditional on the combat-scope
decision.

## Preempted stuck-points + mitigations

- **Ordering (Stage 1):** if a verb's behavior shifts, it's a phase-placement bug
  — replay catches it; fix the `.before()/.after()`, don't revert the decomposition.
- **Bosses (Stage 5):** keep their special parts as components; the spine owns the
  body, not the encounter logic. If a boss fights the body unification, isolate
  the special part rather than abandoning the stage.
- **Projectile-charge player gate / `desired_vel` dual meaning (Stage 7):** combat,
  deferred unless it blocks movement unification.
- **Rendering anchor/baseline (Stage 4):** the player's bespoke sprite anchor +
  crouch-row hack must be reproduced via its catalog tuning; diff the player
  visual before/after.
- **Replay divergence for non-players (Stage 3+):** expected — do NOT chase
  byte-identity for enemies; pin the new intended behavior instead.
- **The 4-cluster → 1 migration (Stage 2):** lean on the compiler; it's wide but
  mechanical.
- **VM disk fills:** `rm -rf <target>/debug/incremental` (regenerable) is the safe
  quick win — learned this run (47G freed).

## Rollback

Every stage is a commit (or few) on `main`. Any stage reverts independently.
The player-feel guard (replay) catches a player regression immediately. Jon can
review the cumulative diff + the run log and keep/refine/revert at any granularity.

## Decisions (CONFIRMED by Jon)

- **Player feel:** IMPROVE OPPORTUNISTICALLY. I may tighten/improve player
  movement feel while decomposing; replay may diverge *intentionally*. Replay
  drops from "player-identity gate" to "tool": it confirms changes I *intend* to
  be neutral, and when I improve feel I let it diverge, pin the new behavior with
  a focused test, and flag the change in the run log for review. Be deliberate —
  flagged feel changes, not accidental ones.
- **Combat scope:** MOVEMENT + COMBAT. Stage 7 (combat unification — attacks,
  projectile-charge, ActionSet effects, the `brain.is_player()` gate, the
  `desired_vel` axis/velocity split) is IN this run.
- **Bosses:** INCLUDE, DO LAST (Stage 5), after player/enemy/NPC land clean.
- **End depth:** through Stage 6 (player peeled to input+camera+HUD) — the
  non-player-centric end state.

This is the full run: every actor type onto one spine, combat unified, player
reduced to input+camera+HUD. Large + reviewed as a cumulative diff on `main`;
keep the run log live so Jon can step away and review the trail.
