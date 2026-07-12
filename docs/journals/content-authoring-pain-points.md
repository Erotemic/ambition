# Content-authoring pain points — stochastic parrot run

A live log of friction encountered while adding ONE new character (the stochastic
parrot) end-to-end: friendly cove NPC + aggressive sky enemies. The goal is to
surface what makes adding content hard so we can refactor toward "author a new
character blind, in data, with a headless test that proves it."

Format: each entry = the friction + why it bites + a candidate fix.

## Pain points

### P1 — Two parallel rosters: `character_catalog.ron` vs `enemy_archetypes.ron`
- A character's sprite/body/hall-tier/default-brain lives in `character_catalog.ron`
  (keyed by `character_id`), but an enemy's combat stats live in a SEPARATE roster
  `enemy_archetypes.ron` (keyed by a brain string used by `EnemyBrain::Custom`).
  To make one creature both a friendly NPC and an aggressive enemy you touch BOTH
  files and keep a name in sync between them.
- Why it bites: no single source of truth for "a character"; the two can drift, and
  it's non-obvious which file owns what. Adding content means hunting across rosters.
- Candidate fix: let the catalog entry own (optionally) the combat stats too, or let
  an enemy archetype reference a `character_id` for its visuals instead of relying on
  name-matching. (See P2.)

### P2 — Enemy → sprite resolves by DISPLAY-NAME string match
- A spawned enemy gets its animated spritesheet via
  `CharacterSpriteAssets::npc_asset_for_name(display_name)` — i.e. the enemy's
  `display_name` string must EXACTLY equal the catalog entry's `display_name`, or it
  silently falls back to a generic sheet (warns once). (render `actors/mod.rs:243-275`.)
- Why it bites: a fragile, stringly-typed join. A decorated/variant name ("Parrot
  (sky)") breaks the sprite with only a log warning — invisible when authoring blind.
- Candidate fix: spawns should carry a `character_id` (a real key), not lean on
  display-name equality, to bind visuals.

### P3 — Hall-of-characters regen is a two-step manual command pair
- After a catalog edit you must run `generate_hall_of_characters` AND THEN
  `area_authoring <spec> --replace-existing`. Forgetting the second step leaves the
  hall stale with no error. (Mitigated: `docs/recipes/adding-a-character.md` documents
  both steps accurately.)
- Candidate fix: a single `regen_hall.sh` (or fold the apply into the generator).

### P4 — LDtk placement is hand-picked pixels + iterate-on-overlap
- Placing a spawn means choosing literal `px` coords, applying, running `validate`,
  reading "overlap within 4px" warnings, then `entity move` to nudge — a slow loop.
  The `entity move` spec key is `target:` (not `match:` like `set-field`), an
  inconsistency that cost a trial-and-error round.
- Candidate fix: a "place relative to <iid>/empty-floor" helper that auto-avoids
  overlaps; unify the entity-edit spec vocabulary (`target` everywhere).

### P5 — Attack patterns are limited to the fixed brain-template enum
- The aggressive parrot had to borrow `brain_template: Shark` (the only aerial
  pursuit brain). There's no aerial-melee-diver template, and the rich attack
  animations (`dive_bomb`, `hover_peck`, `banked_strafe`) aren't bound to any action
  — only `slash` is, via `action.melee.primary`. New movement/attack feels require a
  Rust brain-template addition, not data.
- Candidate fix (the ambitious + refactor commits): a data-authorable patrol/attack
  pattern (waypoints / phases) so a parrot's "fly–land–walk–bark" and "dive–strafe"
  are content, plus animation-binding keys for the extra attack rows.

### P6 — NPC brains ignore the catalog `default_brain` (hardcoded Patrol/StandStill)
- `NpcRuntime::build_brain` (features/npcs.rs) picks `Patrol` if `patrol_radius > 0`
  else `StandStill` — it NEVER reads the catalog `default_brain` preset. So a catalog
  row's `default_brain` is dead for an LDtk `NpcSpawn`; you cannot give a placed NPC a
  richer brain (e.g. the new lively `Aerial`) from data. (Enemies DO honor their
  archetype `brain_template` via `enemy_default_brain` — only the NPC path is stunted.)
- Also: the `Npc` interaction kind carries no `character_id`, so the cluster can't even
  resolve the catalog row to read `body_kind`/brain at spawn — the join is lost.
- Why it bites: the friendly parrot can't be authored as a flyer in data; it's stuck on
  the grounded Patrol brain regardless of its catalog row. This is the single biggest
  blocker to "author a new character's behavior blind, in data."
- Candidate fix (commit 3): thread `character_id` onto the NPC interaction, make
  `build_brain` resolve the catalog `default_brain` preset, and let `body_kind: Floating`
  zero gravity — so an `Aerial` peaceful NPC actually flies. THIS is the refactor that
  makes the friendly parrot's lively flight a data row.

### P7 — Hall regen duplicates the reciprocal hub→hall door
- `area_authoring --replace-existing` re-adds the hub→hall `LoadingZone` without
  noticing the existing one, producing two zones sharing id `hall_of_characters_door`
  → fails content-graph validation. Cost a separate fix commit.
- Candidate fix: the hall regen should upsert (dedup by id) its reciprocal door.

### P8 — NPC brains tick with `sim_time = 0.0` (hardcoded)
- `update_ecs_npcs` passes `sim_time = 0.0` into the brain ("NPC brains run Patrol/
  StandStill, which don't read the clock"). Any sim-time-driven NPC brain — like the
  new lively `Aerial` — gets a frozen clock (no waypoint variety, no dwell timing).
- Why it bites: blocks giving an NPC a time-based behavior. Pairs with P6: even once
  NPC brains are catalog-driven, they need the real sim clock to come alive.
- Candidate fix (commit 3): thread the real sim time into `update_ecs_npcs`.

## Commit 2 — ambitious behavior (what landed, what deferred)
- **New `Aerial` brain** (`brain/state_machine`): one pure, deterministic policy with
  two faces by `aggressiveness` — a lively peaceful bird (perch ↔ fly ↔ walk, drops
  beside the player to talk) and a hostile dive-bomber (stalk → dive → peck → recover).
  Captures its anchor from `actor_pos` on tick 1 (no spawn coord threading). Verified
  by 4 headless integration tests (flight, perch, talk-landing, the dive cycle).
- **Aggressive sky parrots** use it now (`brain_template: Aerial`) — they're already
  `is_aerial` (gravity-free), so the dive works end-to-end.
- **NPC idle barks** (`tick_npc_idle_barks`): ambient ~6–10s chatter; the parrot is the
  first user (stochastic-parrot riffs).
- **DEFERRED to commit 3 (the refactor):** the FRIENDLY parrot still flies only after
  P6 + P8 are fixed (catalog-driven NPC brains + real NPC sim clock + `Floating` ⇒
  gravity-free). That refactor is exactly "make the lively flyer a data row," so it is
  the natural payload of commit 3 rather than a special-case hack here.

## Commit 3 — the refactor (data-driven NPC behavior)
Fixes the keystone blockers (P6 + P8 + the `Floating` flag) so a placed NPC's
behavior comes from its catalog row, not hardcoded Rust. The payoff IS the feature:
the cove parrot now flies, authored entirely in data.

- **`character_id` threaded onto `InteractionKind::Npc`** — the catalog join now
  survives LDtk → spawn (was dropped after resolving the display name). (P6)
- **`NpcRuntime::build_brain` is catalog-aware** — a catalog row asking for a RICH
  brain (anything past Patrol/StandStill, e.g. `Aerial`) is honored via
  `character_roster::default_brain_for_character_id`. Plain Patrol/StandStill rows
  still use the legacy `patrol_radius` heuristic, so every existing NPC is unchanged
  (replay byte-identical, 914 sandbox tests green). (P6)
- **`Floating` body_kind ⇒ gravity-free** — `body_kind_for_character_id` zeroes the
  NPC's `gravity_scale` at spawn, and a new `integrate_velocity_aerial` drives the
  full 2D `desired_vel` with gravity off (mirrors the aerial-enemy integrator). The
  bird actually flies.
- **Real NPC sim clock** — `update_ecs_npcs` accumulates a scaled-dt `Local` clock
  instead of passing `sim_time = 0.0`, so the `Aerial` brain's waypoint/dwell timing
  comes alive (and freezes with pause/bullet-time). (P8)
- **The parrot is now a pure data row**: catalog `default_brain: parrot_lively`
  (a peaceful `Aerial` preset) + `body_kind: Floating`. No per-parrot Rust.

What's STILL not data (future): P1/P2 (two rosters + display-name sprite join), P4
(LDtk placement ergonomics), P7 (hall reciprocal-door dedup), and the extra attack
animations (`dive_bomb`/`hover_peck`/`banked_strafe`) aren't bound to actions yet —
only `slash` fires via `action.melee.primary`. The next refactor target is the
sprite-by-name join (P2): give spawns a `character_id` for visuals too.

## Probe: a brain-driven "player-copy" NPC (the real architectural work)
Jon's test: spawn an NPC that is a COPY OF THE PLAYER and let a brain drive its
full, rich action set (jump/dash/fly/blink/pogo/attack). "If there is pain here
that means there is real architectural work to do." There is. Mapped, not built —
it's a refactor of the player sim systems, too risky to land blind at the tail of
this run.

**What's genuinely unified (the seam is real up to the function boundary):**
- `Brain` (Player | StateMachine) → `ActorControlFrame` → `engine_input_from_actor_control`
  → `InputState` → `integrate_velocity_clusters`. The movement integration is PURE:
  no `With<PlayerEntity>`, no singleton — it reads only `InputState` + cluster refs.
- `ActorControlFrame` already carries EVERY player verb (jump/dash/fly_toggle/blink/
  pogo/fast_fall/shield/special/melee/projectile/aim). An AI brain setting them is
  byte-identical to the human setting them.
- **Possession is the existing proof**: a possessed enemy/NPC is already a non-player
  body driven through the player-brain→ActorControl path (it moves + attacks).

**P9 — the pain: every player sim system is `single_mut()`-keyed to the one player.**
- `game/ambition_app/src/app/sim_systems.rs` runs ~8 systems (movement, abilities,
  attack, dash-cam, …) via `player_q.single_mut()` with `With<PlayerEntity>`. The
  integration FUNCTION is generic, but nothing ITERATES player-bodied entities — so a
  second player-clustered entity is simply never driven.
- Possession sidesteps this by routing the human's input into the EXISTING single
  player slot; it never spawns a second player body. So possession proves the *control
  seam*, not the *multi-body system shape*.
- The real work (a dedicated session): turn the player into "an actor that happens to
  carry the human brain," i.e. generalize those `single_mut()` systems to `iter_mut()`
  over a `PlayerBodied` query, each driven by its OWN `ActorControl` (human brain for
  the player; StateMachine for a clone). This is also the seam that unlocks real
  multiplayer (the architecture-targets memo) and makes possession a brain swap rather
  than an input shim.
- Secondary gaps once the systems iterate: the projectile-CHARGE state machine is
  gated on `brain.is_player()` (brain/mod.rs `emit_player_projectile_tick_messages`),
  and HUD/camera/combo systems are `PlayerEntity`-anchored. Movement/jump/dash/fly/
  blink/pogo/melee need no new code — only the iteration.

**Recommendation:** land this as its own focused run — spawn a `PlayerBodied` clone +
a "demo-movement" StateMachine brain, generalize the sim_systems driver loop, and
assert (headless) the clone jumps/dashes/flies from its brain. That single refactor
is worth more than any one character; it's the keystone the whole actor-unification
was aiming at.

### UPDATE — the seam is PROVEN (headless), and a new wrinkle surfaced
- New `StateMachineCfg::PlayerDemo` brain emits the player's own verbs (run/jump/
  dash/fly) on the shared `ActorControlFrame`. The test
  `player::clone_probe_tests::a_brain_drives_a_full_player_body_through_the_player_movement`
  assembles the 18 player clusters + this brain and drives them through the SAME
  `update_player_clusters` the human uses — the body runs, leaves the ground, and
  flies, with no player-specific path. **The Brain→ActorControlFrame→InputState→
  movement seam is genuinely actor-generic.** Proven.
- **P10 — `desired_vel` has TWO meanings on the shared control frame.** The PLAYER
  path reads `desired_vel` as a normalized stick AXIS in `[-1,1]` (the human brain
  feeds it that way; `engine_input_from_actor_control` copies it straight into
  `InputState.axis_x`, then movement scales by `max_run_speed`). ENEMY brains put a
  px/s VELOCITY in the same field (the enemy integrator uses it directly). So a brain
  that wants to drive a player body must emit axis intent, not velocity — a silent
  unit mismatch on one struct field. Candidate fix: split into `move_axis` (intent)
  vs `move_velocity`, or make both consumers agree on one convention.
- The remaining work is unchanged: the live in-game clone still needs the
  `single_mut()`-keyed player sim systems generalized to iterate (P9). The seam is
  ready; the systems aren't.

### UPDATE 2 — the LIVE in-game clone works (press K)
- `game/ambition_app/src/app/player_clone.rs`: a `PlayerClone` is a non-player
  entity carrying all 18 player movement clusters + a `PlayerDemo` brain + an
  `ActorControl`. `drive_player_clones` ticks its brain → `ActorControl` and runs
  the SHARED `step_motion` kernel (the exact human-player movement
  core) on it. Press **K** in-game to spawn one at the player; it runs/jumps/dashes/
  flies on its own. Live integration test `tests/player_clone_live.rs` asserts it
  moves + leaves the ground in the real app schedule; replay stays byte-identical
  (no clone in the fixture).
- **Design decision (resolving the P9 entanglement pragmatically):** the PRIMARY
  player keeps its full, entangled tick — it owns the global concerns (world clock,
  moving-platform advance, camera, sandbox reset) that a clone must NOT touch. The
  clone gets a focused driver that reuses the per-entity movement core WITHOUT those
  globals. This is not a duplicate integration (the core is shared); it's an honest
  split of "per-body movement" from "the primary player's world responsibilities."
- **The deeper refactor still open (P9'):** decouple those globals into their own
  primary-only systems so ONE loop drives every player-bodied entity (player +
  clones) uniformly. That's the multiplayer shape; the clone driver is the first
  consumer proving the per-body half is clean.
- P10 (the `desired_vel` axis-vs-velocity dual meaning) is handled in the clone path
  by emitting axis intent; the unifying split is still worth doing.

### UPDATE 3 — non-player-centric run (Stages 2–7): the spine + the capability gate
- **P9 projectile-charge gate RESOLVED (Stage 7).** `emit_player_projectile_tick_
  messages` no longer gates on `brain.is_player()` — it gates on a `ChargesProjectiles`
  capability marker (ambition_characters). Only the player carries it today (so behavior is
  byte-identical: same emitter set), but it's now pay-for-use and travels with the
  body, so a possessed actor keeps the charge mechanic. Bosses/enemies that carry a
  `ranged` ActionSet for their OWN projectiles are correctly NOT swept in (the marker
  is distinct from the ActionSet slot).
- **P10 made an explicit, documented contract (Stage 7).** `ActorControlFrame::
  desired_vel`'s doc now states the two encodings precisely and names the BRIDGE:
  `integrate_standard_enemy_body` maps an enemy's px/s velocity onto the shared
  `integrate_normal_spine` via `max_run_speed = |desired_vel.x|`, `axis_x = sign`, so
  both player (axis) and enemy (velocity) reach the SAME grounded spine and agree. The
  velocity-encoding is a bridge, not a second physics path. The clean end-state (every
  brain emits a normalized axis + carries run speed in tuning) is the deferred follow-up.
- **P9' (single_mut globals) status:** the player-centric boundary is already drawn —
  camera/HUD/fx/UI scope on `PrimaryPlayerOnly`, body on `PlayerEntity`, input via
  `ActorControl` for both primary + clone. Collapsing the remaining `single_mut` body
  systems into one loop still needs the globals (platform/shake/reset) peeled out of
  `player_simulation_phase`; left as the documented next step (GUI-feel-sensitive).

## Positives (what already works well)
- **The spawn path is data-driven**: the basic friendly+hostile hookup needed ZERO
  Rust changes — catalog row + archetype row + yarn node + LDtk placement.
- **`docs/recipes/adding-a-character.md`** exists and is accurate (catalog → hall → walk in).
- **The actor unification holds**: one sprite/body serves a peaceful NPC and a hostile
  enemy with no special-casing; hostility is just `attacks_player` + brain choice.
- **Headless coverage is real**: the embedded-LDtk hall test proved the parrot pedestal
  resolves a safe sprite the moment the catalog row landed — caught wiring for free.

